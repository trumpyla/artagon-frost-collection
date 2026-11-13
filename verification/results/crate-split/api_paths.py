#!/usr/bin/env python3
"""List the public item paths reachable from a crate root in rustdoc JSON.

Evidence tool for the crate split (frost-wlt). It is not a Make gate.

usage: api_paths.py <doc-dir> <root-crate> [--methods]

<doc-dir> holds one `<crate>.json` per crate, as written by
`cargo +nightly doc --no-deps` with
`RUSTDOCFLAGS="-Z unstable-options --output-format json"`.

The walk starts at the root module of <root-crate> and follows modules, named
re-exports and glob re-exports, crossing into another crate's JSON when the
re-exported item is defined there. Output is one sorted line per reachable
path: `<kind> <path>`. With --methods, each struct, enum, union and trait is
followed by its inherent method and associated item names and by the traits it
implements, so two layouts can be compared beyond paths.

`#[doc(hidden)]` items are omitted because rustdoc strips them.
"""
import json
import os
import sys

KINDS = (
    "module", "struct", "enum", "union", "trait", "function", "type_alias",
    "constant", "static", "macro", "trait_alias", "proc_macro", "primitive",
)


class Crate:
    def __init__(self, path):
        with open(path) as handle:
            data = json.load(handle)
        self.index = data["index"]
        self.paths = data["paths"]
        self.root = str(data["root"])
        self.external = data["external_crates"]
        self.by_path = {}
        for ident, entry in self.paths.items():
            if entry["crate_id"] == 0:
                self.by_path[tuple(entry["path"])] = ident


class Walker:
    def __init__(self, doc_dir, methods):
        self.doc_dir = doc_dir
        self.crates = {}
        self.methods = methods
        self.lines = set()
        self.unresolved = set()

    def crate(self, name):
        if name not in self.crates:
            path = os.path.join(self.doc_dir, name + ".json")
            self.crates[name] = Crate(path) if os.path.exists(path) else None
        return self.crates[name]

    def resolve(self, crate_name, ident):
        """Returns (crate_name, id) of the defining item, or None."""
        crate = self.crate(crate_name)
        ident = str(ident)
        if ident in crate.index:
            return crate_name, ident
        entry = crate.paths.get(ident)
        if entry is None:
            return None
        other_name = crate.external[str(entry["crate_id"])]["name"]
        other = self.crate(other_name)
        if other is None:
            return None
        target = other.by_path.get(tuple(entry["path"]))
        if target is None or target not in other.index:
            return None
        return other_name, target

    def kind_of(self, item):
        inner = item["inner"]
        for kind in KINDS:
            if kind in inner:
                return kind
        return next(iter(inner))

    def walk_module(self, crate_name, ident, prefix, seen):
        key = (crate_name, ident)
        if key in seen:
            return
        seen = seen | {key}
        crate = self.crate(crate_name)
        module = crate.index[ident]["inner"]["module"]
        for child in module["items"]:
            child = str(child)
            item = crate.index.get(child)
            if item is None:
                continue
            inner = item["inner"]
            if "use" in inner:
                use = inner["use"]
                if use["id"] is None:
                    self.unresolved.add("::".join(prefix + [use["name"]]) + " <- " + use["source"])
                    continue
                found = self.resolve(crate_name, use["id"])
                if found is None:
                    self.unresolved.add("::".join(prefix + [use["name"]]) + " <- " + use["source"])
                    continue
                target_crate, target = found
                target_item = self.crate(target_crate).index[target]
                if use["is_glob"]:
                    if "module" in target_item["inner"]:
                        self.walk_module(target_crate, target, prefix, seen)
                    elif "enum" in target_item["inner"]:
                        self.emit_variants(target_crate, target_item, prefix)
                    continue
                self.emit(target_crate, target, prefix + [use["name"]], seen)
                continue
            if item["visibility"] != "public" or item["name"] is None:
                continue
            self.emit(crate_name, child, prefix + [item["name"]], seen)

    def emit_variants(self, crate_name, item, prefix):
        crate = self.crate(crate_name)
        for variant in item["inner"]["enum"]["variants"]:
            entry = crate.index.get(str(variant))
            if entry is not None:
                self.lines.add("variant " + "::".join(prefix + [entry["name"]]))

    def emit(self, crate_name, ident, path, seen):
        crate = self.crate(crate_name)
        item = crate.index[ident]
        kind = self.kind_of(item)
        text = "::".join(path)
        self.lines.add(kind + " " + text)
        if kind == "module":
            self.walk_module(crate_name, ident, path, seen)
            return
        if kind == "enum":
            self.emit_variants(crate_name, item, path)
        if not self.methods or kind not in ("struct", "enum", "union", "trait"):
            return
        inner = item["inner"][kind]
        if kind == "trait":
            for member in inner["items"]:
                entry = crate.index.get(str(member))
                if entry is not None:
                    self.lines.add("  item " + text + " :: " + entry["name"])
            return
        for impl_id in inner.get("impls", []):
            impl_item = crate.index.get(str(impl_id))
            if impl_item is None:
                continue
            impl = impl_item["inner"]["impl"]
            if impl.get("is_synthetic") or impl.get("blanket_impl") is not None:
                continue
            if impl["trait"] is None:
                for member in impl["items"]:
                    entry = crate.index.get(str(member))
                    if entry is not None and entry["visibility"] == "public":
                        self.lines.add("  item " + text + " :: " + entry["name"])
            else:
                self.lines.add("  impl " + text + " : " + impl["trait"]["path"])


def main():
    args = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    methods = "--methods" in sys.argv
    if len(args) != 2:
        sys.exit(__doc__)
    walker = Walker(args[0], methods)
    root = walker.crate(args[1])
    if root is None:
        sys.exit("no rustdoc JSON for " + args[1])
    walker.walk_module(args[1], root.root, [args[1]], frozenset())
    for line in sorted(walker.lines):
        print(line)
    for line in sorted(walker.unresolved):
        print("extern " + line)


if __name__ == "__main__":
    main()

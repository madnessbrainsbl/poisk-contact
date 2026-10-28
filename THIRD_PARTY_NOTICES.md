# Third-party notices

Rust dependencies are declared in `Cargo.toml` and resolved in `Cargo.lock`.
Their licenses remain with their respective authors. Dependency source and
prebuilt application binaries are not stored in Git. Windows release packages
include dependency license texts in `licenses/` and a dependency inventory in
`THIRD_PARTY_LICENSES.json`.

The following optional companion projects are installed separately:

- [Scrapling](https://github.com/D4Vinci/Scrapling): BSD 3-Clause; installed
  separately in a local Python environment, together with its dependencies.
- [Parser2GIS](https://github.com/Eroloft/parser-2gis-new), a fork of
  [interlark/parser-2gis](https://github.com/interlark/parser-2gis): LGPLv3, as
  stated in its upstream license. The local companion checkout is excluded.
- [Chrome crawler](https://github.com/ir00nman/chrome-crawler): CC BY-NC 4.0, as
  stated by its upstream project. Review its noncommercial restriction before
  using or distributing that companion.

The PoiskContact import adapter does not bundle either companion's code.
This notice does not change their licenses or grant rights to the original
PoiskContact source.

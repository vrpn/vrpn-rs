# VRPN-RS

An async-capable port of the [VRPN][] protocol to [Rust][].

Maintained at <https://github.com/vrpn/vrpn-rs>

Currently **highly experimental** and not entirely operational.

## Usage

This library uses [Rust 2018](https://rust-lang-nursery.github.io/edition-guide/rust-2018/index.html),
and thus, as of 14-Nov-2018, requires the use of the "beta" channel.

Add `vrpn` as a dependency to your `Cargo.toml` file:

```toml
[dependency]
vrpn = "0.1.0"
```

Then add the following to your crate's root:

```rust
extern crate vrpn;
```

Right now, all the top-level APIs for connections/endpoints use [Tokio][] for async IO,
but most of the project is independent of Tokio, so an alternative IO integration
could be created.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md)
for details on our code of conduct, and the process for submitting pull requests to us.

## Primary Authors and Contributors

- **Ryan Pavlik** - *Initial work* - @rpavlik on many platforms

## License

The license for most of the code in this repository is the [Boost Software License 1.0][BSL] -
this is a very permissive free-software licence also used by the mainline VRPN repository.
This license is used to permit free interchange of code between this codebase and the
mainline C++ codebase.

One file (as of the time of this writing) is under the OSI-approved MIT license,
because it is based closely on some sample code from the [Tokio async framework for Rust][Tokio].

Dependencies used via Cargo have their own licenses.

All files have SPDX-License-Identifier tags.

All contributions will be considered to be under the license noted in the changed file's or files'
SPDX tags.

## Acknowledgments

- [Collabora](https://collabora.com) for supporting Ryan's development and maintenance of this code
  in the course of his work.
- Thanks and acknowledgements to [Russ M. Taylor, III][Russ],
  and the other authors and contributors of the [VRPN][] package,
  for pioneering a widely-used de-facto standard in input/output,
  useful particularly but not exclusively in immersive computing.
- Thanks to PurpleBooth's excellent template/advice for structuring a
  [README](https://gist.github.com/PurpleBooth/109311bb0361f32d87a2)


[VRPN]: https://github.com/vrpn/vrpn
[Rust]: https://rust-lang.org
[BSL]: https://spdx.org/licenses/BSL-1.0
[Tokio]: https://tokio.rs
[Russ]: https://www.cs.unc.edu/~taylorr/

---

## Copyright and License for this README.md file

For this file only:

> Initially written by Ryan Pavlik. Copyright 2018 Collabora, Ltd.
>
> SPDX-License-Identifier: CC-BY-4.0

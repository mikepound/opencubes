# How to run

To run this project, install rust (doing so through [rustup] is the most straightforward)

Then, in a terminal or command window, navigate to the `opencubes/rust` directory and run:

```shell
cargo run --release -- run <n>
```

where n is the count of cubes for which to calculate the amount of unique polycubes.

For more info, run:

```shell
cargo run --release -- --help
```

[rustup]: https://rustup.rs/
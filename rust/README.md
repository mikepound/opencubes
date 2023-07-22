# How to run

To run this project, install rust (doing so through [rustup] is the most straightforward)

Then, in a terminal or command window, navigate to the `opencubes/rust` directory and run:

```shell
cargo run --release -- enumerate <n>
```

where n is the count of cubes for which to calculate the amount of unique polycubes.

enumeration flags
```
Usage: opencubes enumerate [OPTIONS] <N>

Arguments:
  <N>  The N value for which to calculate all unique polycubes

Options:
  -p, --no-parallelism
          Disable parallelism
  -c, --no-cache
          Don't use the cache
  -z, --cache-compression <CACHE_COMPRESSION>
          Compress written cache files [default: none] [possible values: none, gzip]
  -m, --mode <MODE>
          [default: standard] [possible values: standard, rotation-reduced, point-list]
  -h, --help
          Print help
```
For more info, run:

```shell
cargo run --release -- --help
```

Commands:
  enumerate  Enumerate polycubes with a specific amount of cubes present
  pcube      Perform operations on pcube files
  help       Print this message or the help of the given subcommand(s)

[rustup]: https://rustup.rs/

## Performance
- Times are cumulative (includes time to calculate all subsets from n=3).

- Times are measured with a large sample set of 1 run in an environment with many background processes of varying resource intensiveness throughout and rounded to 3 sig figs

- No cache files used.

- Working with Low% speedrun rules - more cubes is always better no matter the time,
a faster time is better than a slower time if cube count is equal.

OoM - Out of Memory - unable to run due to memory limitations but we can dream

NA - didnt try, probably out of memory but had an estimated time in hours so didnt measure

Hardware:
Ryzen 9 7900X
32GB ram @ DDR5-6000

| Git hash | N = 6 | N = 7 | N = 8 | N = 9 | N = 10 | N = 11 | N = 12 | N = 13 | N = 14 | N = 15 | N = 16 | N = 17 | Mode |
| -------- | ----- | ----- | ----- | ----- | ------ | ------ | ------ | ------ | ------ | ------ | ------ | ------ | ---- |
| python | 113ms | 713ms | 5.0s | 37.4s | 239s | 2310s | NA | NA | NA | NA | NA | NA | NA |
| 50b6682 | 0.17ms | 1.37ms | 10.3ms | 73.6ms | 0.643s | 5.86s | 55.5s | OoM | OoM | OoM | OoM | OoM | rotation-reduced |
| afa90ad | 0.184ms | 1.43ms | 11.4ms | 8.41ms | 0.686s | 6.58s | 62.45s | 574s | OoM | OoM | OoM | OoM | rotation-reduced |
| 68090de | 13.2ms | 20.4ms | 37.4ms | 85.3ms | 0.304s | 1.74s | 14.2s | 124s | OoM | OoM | OoM | OoM | point-list + rayon |
| b83f9c6 | 3ms | 4.3ms | 8.58ms | 25.4ms | 0.137s | 0.986s | 8.02s | 66.7s | OoM | OoM | OoM | OoM | point-list + rayon |

## To Do
- implement hashtableless solution that just enumerates

- profile and optimise further

- deduplicate mirrors as well to reduce (optimistically) ~50% memory usage in the
hashset and then test for symatry when counting unique variants


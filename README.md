# Miscellaneous command-line tools

[![Rust build status](https://img.shields.io/github/actions/workflow/status/travisbrown/misccli/ci.yaml)](https://github.com/travisbrown/misccli/actions)
[![Coverage status](https://img.shields.io/codecov/c/github/travisbrown/misccli/main.svg)](https://codecov.io/github/travisbrown/misccli)

Please note that this software is **not** "open source",
but the source is available for use and modification by individuals, non-profit organizations, and worker-owned businesses
(see the [license section](#license) below for details).

## Contents

This is just a bag of stuff. The first thing I've added is a tool for merging sorted files. For example, this command:

```bash
$ time target/release/merge a.csv b.csv | wc
12710167 12710167 1591781524

real    0m10.963s
user    0m14.925s
sys     0m5.488s
```

...should be equivalent to this one:

```bash
$ export LC_ALL=C
$ time sort a.csv b.csv | uniq | wc
12710167 12710167 1591781524

real    0m14.645s
user    0m19.738s
sys     0m3.914s
```

...except slightly faster and with a much smaller memory footprint.

## License

This software is published under the [Anti-Capitalist Software License][acsl] (v. 1.4).

[acsl]: https://anticapitalist.software/

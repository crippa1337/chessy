<div align="center">

  # Svart
  [![License][license-badge]][license-link]
  [![Release][release-badge]][release-link]
  [![Commits][commits-badge]][commits-link]

</div>
A free and open source UCI chess engine written in Rust.

Svart is not a complete chess program and requires a [UCI-compatible graphical user interface](https://www.chessprogramming.org/UCI#GUIs) in order to be used comfortably.


# UCI Options
### Hash
> Megabytes of memory allocated for the [Transposition Table](https://en.wikipedia.org/wiki/Transposition_table).
    

# History

| Version  | CCRL Blitz     | CCRL 40/15     | MCERL        |
| -------- | -------------- | -------------- | ------------ |
| Svart 3  | 2886±20 [#126] | 2811±64 [#145] |              |
| Svart 2  | 2463±20 [#281] | 2462±24 [#283] | 2484 [#152]  |
> Single-CPU lists<br>
> Updated 230410


# Compilation
Compile Svart using [Cargo](https://doc.rust-lang.org/cargo/) in ``/target/release``.

### x86-64-v1

    RUSTFLAGS='-C target-feature=+fxsr,+sse,+sse2' cargo build --release

### x86_64-v2

    RUSTFLAGS='-C target-feature=+fxsr,+sse,+sse2,+cmpxchg16b,+popcnt,+sse3,+sse4.1,+sse4.2,+ssse3' cargo build --release

### x86_64-v3

    RUSTFLAGS='-C target-feature=+fxsr,+sse,+sse2,+cmpxchg16b,+popcnt,+sse3,+sse4.1,+sse4.2,+ssse3,+avx,+avx2,+bmi1,+bmi2,+f16c,+fma,+lzcnt,+movbe' cargo build --release

### Optimized for your system

    RUSTFLAGS='-C target-cpu=native' cargo build --release
    
    
[commits-badge]:https://img.shields.io/github/commits-since/crippa1337/svart/latest?style=for-the-badge
[commits-link]:https://github.com/crippa1337/svart/commits/master
[release-badge]:https://img.shields.io/github/v/release/crippa1337/svart?style=for-the-badge&label=official%20release
[release-link]:https://github.com/crippa1337/svart/releases/latest
[license-badge]:https://img.shields.io/github/license/crippa1337/svart?style=for-the-badge&label=license&color=success
[license-link]:https://github.com/crippa1337/svart/blob/master/LICENSE

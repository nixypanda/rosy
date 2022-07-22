# Rust OS Yay!

rosy is a toy OS written in rust. Mainly it is a follow-along of [this amazing
blog series](https://os.phil-opp.com/) to deepen my understanding of OS
concepts. One major difference that you will notice is the dearth of dependencies
that the blog series makes use of. I instead wanted to do most of the things
myself given that the aim here was not to create a functioning OS but to help
me learn.

Currently it can do the following -
- Print to the screen
- Handle a few CPU Exceptions
- Handles timer interrupts
- Handles Keyboard interrupts
- Has paging support
- Heap allocations
- Serial output
- Extremely basic shell (just echos back what you type)

The code is extensively commented so one can go splunking through the codebase
and hopefully learn a few things. The idea of this project is that it should be
reasonably easy to understand.

## Setup

### For the nix users

Have nix with flake support with direnv

- Clone this repository
- cd into the directory (you will need to do `direnv allow`)
- Execute `cargo install bootimage`

### Others

- Install and setup rust nightly
- Install qemu
- Clone this repository
- cd into the directory
- Execute `cargo install bootimage`

## Running

- `cargo run`

## Running tests

- `cargo test`

## Looking at documentation

- `cargo doc --open`

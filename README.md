# mls_chat
`mls_chat` is a personal project I've been working on over the Summer in my spare time, mostly for fun. Currently, it must be noted the project is **heavily in-progress**, and **not yet functional**. The goal is not necessarily to create a production-ready application for extended public use -- security and cryptography are dangerous topics to claim mastery over without many more years of experience than I have -- but to explore the problem out of curiosity.

The project is based around the [MLS (Messaging Layer Security)](https://messaginglayersecurity.rocks/) protocol, published and IETF standardized in 2023. The protocol outlines an efficient and secure process for using a sort of binary tree of cryptographic key exchanges to provide end(s)-to-end(s) encryption for group messaging, allowing groups to communicate without having to be online synchronously or even at the time of group creation. For more information on how the protocol works, I recommend either reading the protocol document or, as I did originally, reading [the earlier paper on Asynchronous Racheting Trees](https://eprint.iacr.org/2017/666.pdf) on which the protocol was based.

## Important notes and reflections
- The `view.rs` file is **currently only slightly edited** from [this example file](https://github.com/ratatui-org/ratatui/blob/main/examples/user_input.rs) in the ratatui repo. I intend to learn much more about using ratatui to create something more my own, but it's a low priority.
- I'm fairly comfortable with Rust for simple programs, but this is my first attempt to use it for a larger application. Design architectures like MVC, which I got used to in Java, rely heavily on the style of classes and inheritence used in Java, and don't work well with Rust.
- Development on this project has been hindered by how new the MLS protocol is. The implementation library used has very poorly maintained documentation (something I could possibly help fix by the time I'm further along with this project), which has led to a lot of uncertainty of whether I'm using its API correctly. On the bright side, I've had to grow a lot in investigating source code and issue/commit history on the library's Github page to solve these inconsistencies myself.
- The current roadblock is in the communication of messages between points -- I'm fairly new to network programming, and something in my handling of TCP streams or the threads working on them is likely the culprit.

## Dependencies
- **[openmls](https://github.com/openmls/openmls):** Rust implementation of the MLS protocol discussed above
- **[tokio](https://github.com/tokio-rs/tokio):** runtime for asynchronous operations in Rust
- **[clap](https://github.com/clap-rs/clap):** command-line argument parsing
- **[ratatui](https://github.com/ratatui-org/ratatui):** command-line user interface libary, with **[tui-input](https://github.com/sayanarijit/tui-input)** and **[crossterm](https://github.com/crossterm-rs/crossterm)** as a backend
- **[chrono](https://github.com/chronotope/chrono):** for in-message timestamps

## Running the project
While the project's functionality is very limited, I realize you may still want to see. Currently, the client application won't be happy if there's no server for it to connect to. To run the server, from within the project directory (assuming you have Rust/Cargo installed):
```
$ cargo run -- host -p [PORT] -s [SIZE (not-yet-implemented max number of open connections)]
```
And then to join the server as a user:
```
$ cargo run -- join -t [server IP] -p [server port] -i [username/id]
```

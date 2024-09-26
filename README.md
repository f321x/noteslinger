## Quick [Nostr](https://en.wikipedia.org/wiki/Nostr) posting CLI
CLI application to quickly publish random identity kind 1 Nostr events with PoW. Uses multithreaded hashing and automatically publishes to hardcoded relays.

### Usage
```
./application_name message pow_target
./noteslinger "Your Braindump" 24
```

### Installation
1. Clone this repository:
```
git clone https://github.com/f321x/noteslinger
```
2. Install the Rust compiler (if not installed)
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
3. Compile the application
```
RUSTFLAGS="-C target-cpu=native" cargo build --release
```
Now the binary is available in ./target/release/noteslinger

### Shell alias
To make the binary quickly available you can set a shell alias to the file:
#### Bash shell alias
```
echo "alias wired='~/path/to/wired-pow-poster/target/release/noteslinger'" >> ~/.bashrc && source ~/.bashrc
```
#### ZSH shell alias
```
echo "alias wired='~/path/to/wired-pow-poster/target/release/noteslinger'" >> ~/.zshrc && source ~/.zshrc
```
The application can then be started just by typing ```wired "Hello Nostr!" pow[int]``` in the shell.

# AzureEmailChecker

Tool written in Rust to enumerate the valid email addresses of an Azure/Office 365 Tenant.

It is mutil threaded and **makes no connection attempts**.

It supports validation of a single email address or a list of emails from a file, with the option of saving valid results in an output file. 

## usage

```
Usage: azure_email_checker [OPTIONS]

Options:
  -e, --email <EMAIL>    Email address to be validated
  -f, --file <FILE>      File containing email addresses to be validated, one per line
  -o, --output <OUTPUT>  Output file for valid addresses
  -v, --verbose          Enables 'VALID' and 'INVALID' results to be displayed in the terminal
  -h, --help             Print help
  -V, --version          Print version

Examples:
  ./azure_email_checker -e emailalonetocheck@domain.com -v
  ./azure_email_checker -f emails.txt -o validemails.txt
```

## prerequisites

- [Rust](https://www.rust-lang.org/tools/install)

## compile

```
sudo apt update && sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu
```

Linux:
```
cargo build --release
```

Windows: 
```
cargo build --release --target x86_64-pc-windows-gnu
```

## credits

- Technique originally discovered by grimhacker and described on this [blog](https://grimhacker.com/2017/07/24/office365-activesync-username-enumeration/)
- [o365creeper](https://github.com/LMGsec/o365creeper/tree/master) the python2.7 tool that motivated this Rust renovation

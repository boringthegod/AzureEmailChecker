use clap::Parser;
use regex::Regex;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufReader, AsyncBufReadExt};
use std::path::Path;
use std::sync::Arc;
use futures::future::join_all;
use colored::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use clap::CommandFactory;

/// Validates email addresses with Azure / Office 365 without submitting connection attempts.
#[derive(Parser, Debug)]
#[command(name = "AzureEmailChecker", version = "1.0", author = "boring", about = "Checks whether an email is valid or not on Microsoft / Azure")]
struct Args {
    /// Email address to be validated.
    #[arg(short, long)]
    email: Option<String>,

    /// File containing email addresses to be validated, one per line.
    #[arg(short, long)]
    file: Option<String>,

    /// Output file for valid addresses.
    #[arg(short, long)]
    output: Option<String>,

    /// Enables 'VALID' and 'INVALID' results to be displayed in the terminal.
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    verbose: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.email.is_none() && args.file.is_none() {
        Args::command().print_help().expect("Failed to print help");
        println!();
        std::process::exit(0);
    }
    let client = Arc::new(Client::new());
    let regex_valid = Regex::new(r#""IfExistsResult":0"#).unwrap();
    let regex_invalid = Regex::new(r#""IfExistsResult":1"#).unwrap();
    let valid_emails_count = Arc::new(AtomicUsize::new(0));
    let total_emails = Arc::new(AtomicUsize::new(0));

    let mut tasks = vec![];

    if let Some(email) = args.email {
        total_emails.fetch_add(1, Ordering::SeqCst);
        tasks.push(tokio::spawn(check_email(
            client.clone(),
            email,
            regex_valid.clone(),
            regex_invalid.clone(),
            args.output.clone(),
            args.verbose,
            valid_emails_count.clone(),
        )));
    } else if let Some(file_path) = args.file {
        let lines = read_lines(&file_path).await.expect("Failed to read lines");
        total_emails.fetch_add(lines.len(), Ordering::SeqCst);
        for email in lines {
            tasks.push(tokio::spawn(check_email(
                client.clone(),
                email,
                regex_valid.clone(),
                regex_invalid.clone(),
                args.output.clone(),
                args.verbose,
                valid_emails_count.clone(),
            )));
        }
    }

    join_all(tasks).await;

    println!("Validation completed: {} valid emails out of {} processed. Results saved to '{}'", valid_emails_count.load(Ordering::SeqCst), total_emails.load(Ordering::SeqCst), args.output.unwrap_or_else(|| "No file specified".to_string()));
}

async fn check_email(
    client: Arc<Client>,
    email: String,
    regex_valid: Regex,
    regex_invalid: Regex,
    output_path: Option<String>,
    verbose: bool,
    valid_emails_count: Arc<AtomicUsize>,
) {
    let url = "https://login.microsoftonline.com/common/GetCredentialType";
    let body = format!(r#"{{"Username":"{}"}}"#, email);
    let res = client.post(url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .expect("Failed to send request")
        .text()
        .await
        .expect("Failed to read response text");

    if regex_invalid.is_match(&res) {
        if verbose {
            println!("{} - {}", email, "INVALID".red());
        }
    } else if regex_valid.is_match(&res) {
        if verbose {
            println!("{} - {}", email, "VALID".green());
        }
        valid_emails_count.fetch_add(1, Ordering::SeqCst);
        if let Some(path) = &output_path {
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .await
                .expect("Failed to open output file");
            let content = format!("{}\n", email);
            file.write_all(content.as_bytes()).await.expect("Failed to write to file");
        }
    }
}

async fn read_lines<P: AsRef<Path>>(filename: P) -> Result<Vec<String>, std::io::Error> {
    let file = File::open(filename).await?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    let mut lines_stream = reader.lines();
    while let Some(line) = lines_stream.next_line().await? {
        lines.push(line);
    }

    Ok(lines)
}

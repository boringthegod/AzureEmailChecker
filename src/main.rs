use clap::Parser;
use regex::Regex;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{BufReader, AsyncBufReadExt};
use std::path::Path;
use std::sync::Arc;
use futures::future::join_all;
use colored::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use clap::CommandFactory;

/// Validates email addresses with Azure / Office 365 without submitting connection attempts.
#[derive(Parser, Debug)]
#[command(name = "AzureEmailChecker", version = "1.1", author = "boring", about = "Checks whether an email is valid or not on Microsoft / Azure")]
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

    /// Output CSV file for valid addresses
    #[arg(short = 'c', long = "csv")]
    csv: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let is_single_check = args.email.is_some();
    let email_clone = args.email.clone();
    if args.email.is_none() && args.file.is_none() {
        Args::command().print_help().expect("Failed to print help");
        println!();
        std::process::exit(0);
    }
    let client = Arc::new(Client::new());
    let regex_valid = Regex::new(r#""IfExistsResult":0"#).unwrap();
    let regex_invalid = Regex::new(r#""IfExistsResult":1"#).unwrap();
    let total_emails = Arc::new(AtomicUsize::new(0));

    let mut tasks = vec![];

    if let Some(email) = args.email {
        total_emails.fetch_add(1, Ordering::SeqCst);
        tasks.push(tokio::spawn(check_email(
            client.clone(),
            email,
            regex_valid.clone(),
            regex_invalid.clone(),
            args.verbose,
            is_single_check,
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
                args.verbose,
                is_single_check,
            )));
        }
    }

    let results = join_all(tasks).await;

    let valid_emails: Vec<String> = results.into_iter()
    .filter_map(|task_result| {
        if let Ok(Ok(Some(email))) = task_result {
            Some(email)
        } else {
            None
        }
    })
    .collect();

    if let Some(csv_path) = args.csv {
        write_to_csv(&csv_path, &valid_emails).await.expect("Failed to write to CSV");
    }
    
    let valid_emails_count = valid_emails.len(); 

    if let Some(output_path) = args.output.as_ref() {
        write_to_text(output_path, &valid_emails).await.expect("Failed to write to text file");
    }
    
    
    if is_single_check {
        if let Some(email) = email_clone {
            println!("Checking completed for: {}", email);
        }
    } else {
        println!("Validation completed: {} valid emails out of {} processed. Results saved to '{}'", valid_emails_count, total_emails.load(Ordering::SeqCst), args.output.unwrap_or_else(|| "No file specified".to_string()));
    }
}

async fn check_email(
    client: Arc<Client>,
    email: String,
    regex_valid: Regex,
    regex_invalid: Regex,
    verbose: bool,
    is_single_check: bool,
) -> Result<Option<String>, Box<dyn std::error::Error + Send>> {
    let url = "https://login.microsoftonline.com/common/GetCredentialType";
    let body = format!(r#"{{"Username":"{}"}}"#, email);
    let res = client.post(url)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?
        .text()
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    if regex_invalid.is_match(&res) {
        if verbose || is_single_check {
            println!("{} - {}", email, "INVALID".red());
        }
        Ok(None)
    } else if regex_valid.is_match(&res) {
        if verbose || is_single_check {
            println!("{} - {}", email, "VALID".green());
        }
        Ok(Some(email))
    } else {
        Ok(None) 
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

async fn write_to_csv(path: &str, emails: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;

    for (index, email) in emails.iter().enumerate() {
        wtr.write_record(&[&index.to_string(), email])?;
    }

    wtr.flush()?;
    Ok(())
}

async fn write_to_text(path: &str, emails: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncWriteExt, BufWriter};
    let file = File::create(path).await?;
    let mut writer = BufWriter::new(file);

    for email in emails {
        writer.write_all(email.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }

    writer.flush().await?;
    Ok(())
}
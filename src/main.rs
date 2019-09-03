extern crate chrono;
extern crate clap;
extern crate humantime;

use chrono::format::ParseError;
use clap::App;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let default_duration = chrono::Duration::seconds(10);
    let usage = format!(
        "-i, --interval=[DURATION_LITERAL]   'Samples log file in given interval; default is {}s'
                        <INPUT>                                     'Sets input file'",
        default_duration.num_seconds()
    );

    let args = App::new("muncher")
        .version("0.1")
        .author("Casey Waldren")
        .about("Summarizes log files as a simple frequency chart")
        .args_from_usage(&usage)
        .get_matches();

    let interval = args
        .value_of("interval")
        .and_then(parse_duration)
        .unwrap_or(default_duration);

    let path = args.value_of("INPUT").unwrap(); //safe because INPUT is a required argument

    ::std::process::exit(match app_run(interval, path) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}

//extracts a chrono::Duration from a human-readable string, such as "10s"
fn parse_duration(argument_string: &str) -> Option<chrono::Duration> {
    match argument_string.parse::<humantime::Duration>() {
        Ok(parsed_duration) => {
            if parsed_duration.as_nanos() == 0 {
                println!("Warning: zero duration given; using default");
                None
            } else {
                chrono::Duration::from_std(parsed_duration.into()).ok()
            }
        }
        Err(_) => {
            println!("Warning: invalid sample duration given; using default");
            println!("Hint: you may use values like '10s' or '3h'");
            None
        }
    }
}

fn app_run(interval: chrono::Duration, path: &str) -> std::io::Result<()> {
    let input = File::open(path)?;
    let buffered = BufReader::new(input);

    let mut buckets: HashMap<i64, u64> = HashMap::new();
    let mut anchor: Option<chrono::DateTime<chrono::Utc>> = None;

    for line in buffered.lines() {
        match extract_date(line?) {
            Ok(date) => {
                if anchor.is_none() {
                    anchor = Some(date);
                }
                bucket_insert(anchor.unwrap(), &mut buckets, date, interval);
            }
            Err(_) => {}
        }
    }

    render_graph(&buckets, &interval);
    Ok(())
}

fn bucket_insert(
    anchor: chrono::DateTime<chrono::Utc>,
    buckets: &mut HashMap<i64, u64>,
    date: chrono::DateTime<chrono::Utc>,
    interval: chrono::Duration,
) {
    let bucket = (date - anchor).num_milliseconds() / interval.num_milliseconds();
    let val = buckets.entry(bucket).or_insert(0);
    *val += 1;
}

fn extract_date(line: String) -> Result<chrono::DateTime<chrono::Utc>, ParseError> {
    let tokens = line.split("\t").take(1).collect::<Vec<_>>();
    tokens[0].parse::<chrono::DateTime<chrono::Utc>>()
}

fn render_graph(stats: &HashMap<i64, u64>, interval: &chrono::Duration) {
    if let Some(&max) = stats.values().max() {
        for i in 0..max {
            render_row(i as u64, stats, max);
        }
        render_baseline(stats);
    } else {
        println!("It appears that no valid log lines are present");
    }
}

fn render_baseline(stats: &HashMap<i64, u64>) {
    let max = stats.keys().max().unwrap();
    let line = "-".repeat(*max as usize + 1);
    println!("{}", line);
}

fn render_row(row: u64, stats: &HashMap<i64, u64>, height: u64) {
    let icon = "|";

    let max = stats.keys().max().unwrap();
    let line: String = (0..=*max)
        .map(|x| match stats.get(&x) {
            Some(&value) => {
                //println!("There are {} events in bucket {}", value, x);
                if value >= height - row {
                    icon
                } else {
                    " "
                }
            }
            None => " ",
        })
        .collect();

    println!("{}", line);
}

extern crate colored;
extern crate json;
extern crate chrono;

use chrono::prelude::*;

use std::io;
use std::io::BufRead;
use colored::*;

enum Level {
    TRACE,
    DEBUG,
    INFO,
    WARN,
    ERROR,
    FATAL,
}

struct Log {
    timestamp: DateTime<Utc>,
    logger_level: Level,
    logger_name: String,
    thread_name: String,
    message: String,
}

fn split_once(in_string: &str) -> Option<(&str, &str)> {
    let mut splitter = in_string.splitn(2, "| ");
    let first = splitter.next()?;
    let second = splitter.next()?;
    Some((first, second))
}

// TODO: Abbreviate java style
fn abbreviate_logger_name(class: &str, count: usize) -> String {
    let utf16_vec: Vec<u16> = class.encode_utf16().collect();
    if utf16_vec.len() <= count {
        return class.to_owned();
        // TODO: bail out early?
    } else {
        // TODO: Properly iterate through first characters in package names
        let sub = take_end(utf16_vec.as_ref(), count);
        return from_utf16_java_lossy(sub);
    }
}

fn abbreviate_thread_name(thread_name: &str, count: usize) -> String {
    let utf16_vec: Vec<u16> = thread_name.encode_utf16().collect();
    let sub = take_end(utf16_vec.as_ref(), count);
    return from_utf16_java_lossy(sub);
}

// Note: We don't have to care about chopping bad characters up because java already handles this
fn from_utf16_java_lossy(v: &[u16]) -> String {
    std::char::decode_utf16(v.iter().cloned()).map(|r| r.unwrap_or('?')).collect()
}

fn take_end<T>(vec: &[T], count: usize) -> &[T] {
    if count >= vec.len() {
        vec
    } else {
        &vec[vec.len() - count..]
    }
}

fn parse(jserb: json::JsonValue) -> Option<Log> {
    let timestamp = match jserb["@timestamp"].as_str().or_else(|| jserb["timestamp"].as_str())?.parse::<DateTime<Utc>>() {
        Ok(timestamp) => timestamp,
        Err(_) => return None,
    };
    // let version = jserb["@version"].as_i32()?; // Unused
    let message = jserb["exception"].as_str().or_else(|| jserb["message"].as_str())?.to_owned();
    let logger_name_long = jserb["logger_name"].as_str().or_else(|| jserb["class"].as_str()).or_else(|| jserb["logger"].as_str())?;
    let logger_name = abbreviate_logger_name(logger_name_long, 40);
    let logger_level = match jserb["level"].as_str()? {
        "TRACE" => Level::TRACE,
        "DEBUG" => Level::DEBUG,
        "INFO" => Level::INFO,
        "WARN" => Level::WARN,
        "ERROR" => Level::ERROR,
        "FATAL" => Level::FATAL,
        _ => return None,
    };
    let thread_name_long = jserb["thread_name"].as_str().or_else(|| jserb["thread"].as_str())?;
    let thread_name: String = abbreviate_thread_name(thread_name_long, 15);
    Some(Log {
        timestamp,
        logger_level,
        logger_name,
        thread_name,
        message,
    })
}

fn parse_line(line: &str) -> Option<(String, Log)> {
    let (app, line2) = split_once(line)?;
    let jserb = match json::parse(line2) {
        Ok(jserb) => jserb,
        Err(_) => return None,
    };
    let log = parse(jserb)?;
    Some((app.to_owned(), log))
}

fn main() {
    let stdin = io::stdin();
    let handle = stdin.lock();
    for line in handle.lines() {
        match line {
            Ok(line) => {
                // let parsed = json::parse(line);
                match parse_line(line.as_str()) {
                    Some((app, log)) => {
                        let level = match log.logger_level {
                            Level::TRACE => "TRACE".green(),
                            Level::DEBUG => "DEBUG".green(),
                            Level::INFO => "INFO".green(),
                            Level::WARN => "WARN".yellow(),
                            Level::ERROR => "ERROR".red(),
                            Level::FATAL => "FATAL".red(),
                        };
                        println!("{}| {} {:>5} {} {} {:<40} {} {}",
                                 app, // 0
                                 // formatted as yyyy-MM-dd HH:mm:ss.SSS
                                 log.timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string().dimmed(),
                                 level,
                                 "---".dimmed(),
                                 format!("[{:>15}]", log.thread_name).dimmed(),
                                 log.logger_name.cyan(),
                                 ":".dimmed(),
                                 log.message)
                    }
                    None => {
                        println!("{}", line)
                    }
                }
            }
            Err(err) => {
                panic!("IO error: {}", err)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use abbreviate_thread_name;

    #[test]
    fn thread_name_ok() {
        let test_str = "NIOServerCxn.Factory:0.0.0.0/0.0.0.0:2181";
        let end = abbreviate_thread_name(test_str, 15);
        assert_eq!(end, ".0/0.0.0.0:2181");
    }

    #[test]
    fn thread_name_bad() {
        let test_str = "---👍0000-111122222";
        let end = abbreviate_thread_name(test_str, 15);
        assert_eq!(end, "?0000-111122222");
    }
}

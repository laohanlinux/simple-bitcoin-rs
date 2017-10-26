extern crate slog;
extern crate slog_term;
extern crate slog_async;

use self::slog::*;
use std::io;

lazy_static! {
    pub static ref LOG: Logger = {
        let plain = slog_term::PlainSyncDecorator::new(io::stdout());
        let logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());
        info!(logger, "finish init logger env");
        logger
    }; 
}

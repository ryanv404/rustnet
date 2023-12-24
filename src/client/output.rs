use std::io::{BufWriter, StdoutLock, Write};
use std::process;

use crate::{NetResult, Request, Response};
use crate::colors::{CLR, RED};

/// A trait containing methods for printing CLI argument errors.
pub trait WriteCliError {
    /// Prints unknown argument error message and exits the program.
    fn unknown_arg(&self, name: &str) {
        eprintln!("{RED}Unknown argument: `{name}`{CLR}");
        process::exit(1);
    }

    /// Prints missing argument error message and exits the program.
    fn missing_arg(&self, name: &str) {
        eprintln!("{RED}Missing `{name}` argument.{CLR}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    fn invalid_arg(&self, name: &str, arg: &str) {
        eprintln!("{RED}Invalid `{name}` argument: \"{arg}\"{CLR}");
        process::exit(1);
    }
}

/// Describes the output style for an HTTP client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Style {
    None,
    PlainReq,
    PlainRes,
    PlainBoth,
    ColorReq,
    ColorRes,
    ColorBoth,
}

impl Style {
    /// Returns true if this `Style` is the `Style::None` variant.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true if this `Style` is a plain variant.
    #[must_use]
    pub const fn is_plain(&self) -> bool {
        matches!(self, Self::PlainReq | Self::PlainRes | Self::PlainBoth)
    }

    /// Returns true if this `Style` is a color variant.
    #[must_use]
    pub const fn is_color(&self) -> bool {
        matches!(self, Self::ColorReq | Self::ColorRes | Self::ColorBoth)
    }

    /// Returns true if this `Style` outputs a request.
    #[must_use]
    pub const fn has_request(&self) -> bool {
        matches!(self, Self::PlainReq
            | Self::ColorReq
            | Self::PlainBoth
            | Self::ColorBoth
        )
    }

    /// Returns true if this `Style` outputs a response.
    #[must_use]
    pub const fn has_response(&self) -> bool {
        matches!(self, Self::PlainRes
            | Self::ColorRes
            | Self::PlainBoth
            | Self::ColorBoth
        )
    }
}

/// The output configuration settings for an HTTP client.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Output {
    pub no_dates: bool,
    pub first_line: Style,
    pub headers: Style,
    pub body: Style,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            no_dates: false,
            first_line: Style::ColorRes,
            headers: Style::ColorRes,
            body: Style::PlainRes
        }
    }
}

impl WriteCliError for Output {}

impl Output {
    /// Returns true if the request line is output.
    #[must_use]
    pub const fn req_line(&self) -> bool {
        matches!(self.first_line, Style::PlainReq
            | Style::PlainBoth
            | Style::ColorReq
            | Style::ColorBoth
        )
    }

    /// Returns true if the request line is output with plain style.
    #[must_use]
    pub const fn req_line_plain(&self) -> bool {
        matches!(self.first_line, Style::PlainReq | Style::PlainBoth)
    }

    /// Returns true if the request line is output with color style.
    #[must_use]
    pub const fn req_line_color(&self) -> bool {
        matches!(self.first_line, Style::ColorReq | Style::ColorBoth)
    }

    /// Returns true if the request headers are output.
    #[must_use]
    pub const fn req_headers(&self) -> bool {
        matches!(self.headers, Style::PlainReq
            | Style::PlainBoth
            | Style::ColorReq
            | Style::ColorBoth)
    }

    /// Returns true if the request headers are output with plain style.
    #[must_use]
    pub const fn req_headers_plain(&self) -> bool {
        matches!(self.headers, Style::PlainReq | Style::PlainBoth)
    }

    /// Returns true if the request headers are output with color style.
    #[must_use]
    pub const fn req_headers_color(&self) -> bool {
        matches!(self.headers, Style::ColorReq | Style::ColorBoth)
    }

    /// Returns true if the request body is output.
    #[must_use]
    pub const fn req_body(&self) -> bool {
        matches!(self.body, Style::PlainReq | Style::PlainBoth)
    }

    /// Returns true if the status line is output.
    #[must_use]
    pub const fn status_line(&self) -> bool {
        matches!(self.first_line, Style::PlainReq
            | Style::PlainBoth
            | Style::ColorReq
            | Style::ColorBoth
        )
    }

    /// Returns true if the status line is output with plain style.
    #[must_use]
    pub const fn status_line_plain(&self) -> bool {
        matches!(self.first_line, Style::PlainRes | Style::PlainBoth)
    }

    /// Returns true if the status line is output with color style.
    #[must_use]
    pub const fn status_line_color(&self) -> bool {
        matches!(self.first_line, Style::ColorRes | Style::ColorBoth)
    }

    /// Returns true if the response headers are output.
    #[must_use]
    pub const fn res_headers(&self) -> bool {
        matches!(self.headers, Style::PlainReq
            | Style::PlainBoth
            | Style::ColorReq
            | Style::ColorBoth
        )
    }

    /// Returns true if the response headers are output with plain style.
    #[must_use]
    pub const fn res_headers_plain(&self) -> bool {
        matches!(self.headers, Style::PlainRes | Style::PlainBoth)
    }

    /// Returns true if the response headers are output with color style.
    #[must_use]
    pub const fn res_headers_color(&self) -> bool {
        matches!(self.headers, Style::ColorRes | Style::ColorBoth)
    }

    /// Returns true if the response body is output.
    #[must_use]
    pub const fn res_body(&self) -> bool {
        matches!(self.body, Style::PlainRes | Style::PlainBoth)
    }

    /// Returns true if a component of both the request and the response are
    /// set to be output.
    #[must_use]
    pub const fn include_separator(&self) -> bool {
        let req = self.req_line()
            || self.req_headers()
            || self.req_body();

        let res = self.status_line()
            || self.res_headers()
            || self.res_body();

        req && res
    }

    /// Clear all styles by setting them to `Style::None`.
    pub fn clear_style(&mut self) {
        self.first_line = Style::None;
        self.headers = Style::None;
        self.body = Style::None;
    }

    /// Set the output style based on the given format string.
    pub fn set_style(&mut self, format_str: &str) {
        // Disable default output style first.
        self.clear_style();

        format_str.chars().for_each(|c| match c {
            'R' if self.first_line.is_none() => {
                self.first_line = Style::ColorReq;
            },
            'R' if self.first_line.has_response() => {
                self.first_line = Style::ColorBoth;
            },
            'H' if self.headers.is_none() => {
                self.headers = Style::ColorReq;
            },
            'H' if self.headers.has_response() => {
                self.headers = Style::ColorBoth;
            },
            'B' if self.body.is_none() => {
                self.body = Style::PlainReq;
            },
            'B' if self.body.has_response() => {
                self.body = Style::PlainBoth;
            },
            's' if self.first_line.is_none() => {
                self.first_line = Style::ColorRes;
            },
            's' if self.first_line.has_request() => {
                self.first_line = Style::ColorBoth;
            },
            'h' if self.headers.is_none() => {
                self.headers = Style::ColorRes;
            },
            'h' if self.headers.has_request() => {
                self.headers = Style::ColorBoth;
            },
            'b' if self.body.is_none() => {
                self.body = Style::PlainRes;
            },
            'b' if self.body.has_request() => {
                self.body = Style::PlainBoth;
            },
            // Ignore quotation marks.
            '\'' | '"' => {},
            _ => self.invalid_arg("--output", format_str),
        });
    }

    /// Sets all color `Style` variants to their plain counterparts.
    pub fn make_plain(&mut self) {
        match self.first_line {
            Style::ColorReq => self.first_line = Style::PlainReq,
            Style::ColorRes => self.first_line = Style::PlainRes,
            Style::ColorBoth => self.first_line = Style::PlainBoth,
            _ => {},
        }

        match self.headers {
            Style::ColorReq => self.headers = Style::PlainReq,
            Style::ColorRes => self.headers = Style::PlainRes,
            Style::ColorBoth => self.headers = Style::PlainBoth,
            _ => {},
        }
    }

    /// Handles the writing of request components to stdout.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the request to stdout fails.
    pub fn write_request(
        &self,
        req: &Request,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        if self.req_line_plain() {
            req.request_line.write_plain(writer)?;
        } else if self.req_line_color() {
            req.request_line.write_color(writer)?;
        }

        if self.req_headers_plain() {
            req.headers.write_plain(writer)?;
        } else if self.req_headers_color() {
            req.headers.write_color(writer)?;
        }

        if self.req_body() && req.body.is_printable() {
            writeln!(writer, "{}", &req.body)?;
        }

        Ok(())
    }

    /// Handles the writing of response components to stdout.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the response to stdout fails.
    pub fn write_response(
        &self,
        do_separator: bool,
        is_head_route: bool,
        res: &Response,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        if do_separator {
            writeln!(writer)?;
        }

        if self.status_line_plain() {
            res.status_line.write_plain(writer)?;
        } else if self.status_line_color() {
            res.status_line.write_color(writer)?;
        }

        if self.res_headers_plain() {
            res.headers.write_plain(writer)?;
        } else if self.res_headers_color() {
            res.headers.write_color(writer)?;
        }

        if !is_head_route
                && self.res_body()
                && res.body.is_printable()
        {
            writeln!(writer, "{}", &res.body)?;
        }

        Ok(())
    }

    /// Sets the "minimal" output style. 
    pub fn set_minimal(&mut self) {
        self.first_line = Style::ColorBoth;
        self.headers = Style::None;
        self.body = Style::None;
    }

    /// Sets the "request" output style. 
    pub fn set_request(&mut self) {
        self.first_line = Style::ColorReq;
        self.headers = Style::ColorReq;
        self.body = Style::PlainReq;
    }

    /// Sets the "verbose" output style. 
    pub fn set_verbose(&mut self) {
        self.first_line = Style::ColorBoth;
        self.headers = Style::ColorBoth;
        self.body = Style::PlainBoth;
    }
}

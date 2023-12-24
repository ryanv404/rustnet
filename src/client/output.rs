use std::io::{BufWriter, Write};
use std::process;

use crate::{Request, Response, NetResult};
use crate::colors::{CLR, RED};

/// A trait containing methods for printing CLI argument errors.
pub trait WriteCliError {
    /// Prints unknown option error message and exits the program.
    fn unknown_opt(&self, name: &str) {
        eprintln!("{RED}Unknown option: `{name}`{CLR}");
        process::exit(1);
    }

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

/// Describes which components are output.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Parts {
    Line,
    Hdrs,
    Body,
    LineHdrs,
    LineBody,
    HdrsBody,
    All,
}

impl Parts {
    /// Returns true if this `Parts` variant includes the first line.
    pub fn includes_first_line(&self) -> bool {
        matches!(
            self,
            Self::Line
                | Self::LineHdrs
                | Self::LineBody
                | Self::All
        )
    }

    /// Returns true if this `Parts` variant includes the headers.
    pub fn includes_headers(&self) -> bool {
        matches!(
            self,
            Self::Hdrs
                | Self::LineHdrs
                | Self::HdrsBody
                | Self::All
        )
    }

    /// Returns true if this `Parts` variant includes the body.
    pub fn includes_body(&self) -> bool {
        matches!(
            self,
            Self::Body
                | Self::LineBody
                | Self::HdrsBody
                | Self::All
        )
    }
}

/// Describes the formatting of `Parts` components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Style {
    None,
    Plain(Parts),
    Color(Parts),
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
        matches!(self, Self::Plain(_))
    }
    
    /// Returns true if this `Style` is a color variant.
    #[must_use]
    pub const fn is_color(&self) -> bool {
        matches!(self, Self::Color(_))
    }

    /// Returns true if this `Style` prints in not `Style::None`.
    #[must_use]
    pub const fn is_printed(&self) -> bool {
        !self.is_none()
    }

    /// Returns true if this `Style` prints the `RequestLine` or `StatusLine`.
    #[must_use]
    pub fn first_line_is_printed(&self) -> bool {
        match self {
            Self::None => false,
            Self::Plain(s) | Self::Color(s) => s.includes_first_line(),
        }
    }

    /// Returns true if this `Style` prints the `Headers`.
    #[must_use]
    pub fn headers_are_printed(&self) -> bool {
        match self {
            Self::None => false,
            Self::Plain(s) | Self::Color(s) => s.includes_headers(),
        }
    }

    /// Returns true if this `Style` prints the `Body`.
    #[must_use]
    pub fn body_is_printed(&self) -> bool {
        match self {
            Self::None => false,
            Self::Plain(s) | Self::Color(s) => s.includes_body(),
        }
    }
}

/// The output configuration settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputStyle {
    pub no_dates: bool,
    pub req_style: Style,
    pub res_style: Style,
}

impl Default for OutputStyle {
    fn default() -> Self {
        Self {
            no_dates: false,
            req_style: Style::None,
            res_style: Style::Color(Parts::All)
        }
    }
}

impl WriteCliError for OutputStyle {}

impl OutputStyle {
    /// Prints the request line if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_request_line<W: Write>(
        &self,
        req: &Request,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        match self.req_style {
            Style::Plain(parts) if parts.includes_first_line() => {
                req.request_line.print_plain(out)
            },
            Style::Color(parts) if parts.includes_first_line() => {
                req.request_line.print_color(out)
            },
            _ => Ok(()),
        }
    }

    /// Prints the status line if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_status_line<W: Write>(
        &self,
        res: &Response,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        match self.res_style {
            Style::Plain(parts) if parts.includes_first_line() => {
                res.status_line.print_plain(out)
            },
            Style::Color(parts) if parts.includes_first_line() => {
                res.status_line.print_color(out)
            },
            _ => Ok(()),
        }
    }

    /// Prints the request headers if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_req_headers<W: Write>(
        &self,
        req: &Request,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        match self.req_style {
            Style::Plain(parts) if parts.includes_headers() => {
                req.headers.print_plain(out)
            },
            Style::Color(parts) if parts.includes_headers() => {
                req.headers.print_color(out)
            },
            _ => Ok(()),
        }
    }

    /// Prints the response headers if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_res_headers<W: Write>(
        &self,
        res: &Response,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        match self.res_style {
            Style::Plain(parts) if parts.includes_headers() => {
                res.headers.print_plain(out)
            },
            Style::Color(parts) if parts.includes_headers() => {
                res.headers.print_color(out)
            },
            _ => Ok(()),
        }
    }

    /// Prints the request body if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_req_body<W: Write>(
        &self,
        req: &Request,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        match self.req_style {
            Style::Plain(parts) | Style::Color(parts) => {
                if parts.includes_body() {
                    writeln!(out, "{}", &req.body)?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// Prints the response body if appropriate for this `OutputStyle`.
    #[must_use]
    pub fn print_res_body<W: Write>(
        &self,
        res: &Response,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        if res.body.is_printable() {
            match self.res_style {
                Style::Plain(parts) | Style::Color(parts) => {
                    if parts.includes_body() {
                        writeln!(out, "{}", &res.body)?;
                    }
                },
                _ => {},
            }
        }

        Ok(())
    }

    /// Returns true if a component of both the request and the response is
    /// printed.
    #[must_use]
    pub const fn include_separator(&self) -> bool {
        self.req_style.is_printed() && self.res_style.is_printed()
    }

    /// Sets the `OutputStyle` for requests and responses based upon a
    /// format string provided by the caller.
    ///
    /// Format string key:
    ///   R: Request line
    ///   H: Request headers
    ///   B: Request body
    ///   s: Status line
    ///   h: Response headers
    ///   b: Response body
    ///   *: "Verbose" style
    ///   r: "Request" style
    pub fn format_str(&mut self, format_str: &str) {
        let mut req_num = 0;
        let mut res_num = 0;

        // Disable default output style first.
        self.clear_styles();

        for c in format_str.chars() {
            match u32::from(c) {
                42 => {
                    // "Verbose" style.
                    self.req_style = Style::Color(Parts::All);
                    self.res_style = Style::Color(Parts::All);
                    return;
                },
                114 => {
                    // "Request" style.
                    self.req_style = Style::Color(Parts::All);
                    self.res_style = Style::None;
                    return;
                },
                // Request styles.
                n if n < 97 => req_num += n,
                // Response styles.
                n => res_num += n,
            }
        }

        self.req_style = match req_num {
            0 => Style::None,
            66 => Style::Color(Parts::Body),
            72 => Style::Color(Parts::Hdrs),
            82 => Style::Color(Parts::Line),
            138 => Style::Color(Parts::HdrsBody),
            148 => Style::Color(Parts::LineBody),
            154 => Style::Color(Parts::LineHdrs),
            220 => Style::Color(Parts::All),
            _ => return self.invalid_arg("--output", format_str),
        };

        self.res_style = match res_num {
            0 => Style::None,
            98 => Style::Color(Parts::Body),
            104 => Style::Color(Parts::Hdrs),
            115 => Style::Color(Parts::Line),
            202 => Style::Color(Parts::HdrsBody),
            213 => Style::Color(Parts::LineBody),
            219 => Style::Color(Parts::LineHdrs),
            317 => Style::Color(Parts::All),
            _ => return self.invalid_arg("--output", format_str),
        };
    }

    /// Converts all `Style::Color` variants to `Style::Plain` variants
    /// without changing the inner `Parts` selection.
    pub fn make_plain(&mut self) {
        if let Style::Color(parts) = self.req_style {
            self.req_style = Style::Plain(parts);
        }

        if let Style::Color(parts) = self.res_style {
            self.res_style = Style::Plain(parts);
        }
    }

    /// Clear all styles by setting them to `Style::None`.
    pub fn clear_styles(&mut self) {
        self.req_style = Style::None;
        self.res_style = Style::None;
    }
}

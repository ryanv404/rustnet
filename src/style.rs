use crate::WriteCliError;

/// ANSI colors.
pub mod colors {
    pub const RED: &str = "\x1b[38;2;255;0;0m";
    pub const GREEN: &str = "\x1b[38;2;0;255;145m";
    pub const YELLOW: &str = "\x1b[38;2;255;255;0m";
    pub const BLUE: &str = "\x1b[38;2;0;170;235m";
    pub const MAGENTA: &str = "\x1b[38;2;255;0;155m";
    pub const CYAN: &str = "\x1b[38;2;0;255;255m";
    pub const ORANGE: &str = "\x1b[38;2;255;128;0m";
    pub const RESET: &str = "\x1b[0m";
}

/// Controls which components are printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Parts {
    None,
    Line,
    Hdrs,
    Body,
    LineHdrs,
    LineBody,
    HdrsBody,
    All,
}

impl Parts {
    /// Returns true if this variant does not print anything.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true if this variant prints all components.
    #[must_use]
    pub const fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Returns true if this variant prints first line.
    #[must_use]
    pub const fn is_first_line(&self) -> bool {
        matches!(
            self,
            Self::Line
                | Self::LineHdrs
                | Self::LineBody
                | Self::All
        )
    }

    /// Returns true if this variant prints the headers.
    #[must_use]
    pub const fn is_headers(&self) -> bool {
        matches!(
            self,
            Self::Hdrs
                | Self::LineHdrs
                | Self::HdrsBody
                | Self::All
        )
    }

    /// Returns true if this variant prints the body.
    #[must_use]
    pub const fn is_body(&self) -> bool {
        matches!(
            self,
            Self::Body
                | Self::LineBody
                | Self::HdrsBody
                | Self::All
        )
    }
}

/// Controls whether the `Parts` are colorized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Plain(Parts),
    Color(Parts),
}

impl Kind {
    /// Returns true if this `Kind` is the `None` variant.
    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.into_parts().is_none()
    }

    /// Returns true if this `Kind` is a `Plain` variant.
    #[must_use]
    pub const fn is_plain(&self) -> bool {
        matches!(*self, Self::Plain(_))
    }
    
    /// Returns true if this `Kind` is a `Color` variant.
    #[must_use]
    pub const fn is_color(&self) -> bool {
        matches!(*self, Self::Color(_))
    }

    /// Returns true if this `Kind` is printed.
    #[must_use]
    pub const fn is_printed(&self) -> bool {
        !self.is_none()
    }

    /// Returns true if this `Kind` prints the request line or
    /// status line.
    #[must_use]
    pub const fn is_first_line(&self) -> bool {
        self.into_parts().is_first_line()
    }

    /// Returns true if this `Kind` prints a plain request line or
    /// status line.
    #[must_use]
    pub const fn is_plain_first_line(&self) -> bool {
        self.is_plain() && self.is_first_line()
    }

    /// Returns true if this `Kind` prints a color request line or
    /// status line.
    #[must_use]
    pub const fn is_color_first_line(&self) -> bool {
        self.is_color() && self.is_first_line()
    }

    /// Returns true if this `Kind` prints the `Headers`.
    #[must_use]
    pub const fn is_headers(&self) -> bool {
        self.into_parts().is_headers()
    }

    /// Returns true if this `Kind` prints plain `Headers`.
    #[must_use]
    pub const fn is_plain_headers(&self) -> bool {
        self.is_plain() && self.is_headers()
    }

    /// Returns true if this `Kind` prints color `Headers`.
    #[must_use]
    pub const fn is_color_headers(&self) -> bool {
        self.is_color() && self.is_headers()
    }

    /// Returns true if this `Kind` prints the `Body`.
    #[must_use]
    pub const fn is_body(&self) -> bool {
        self.into_parts().is_body()
    }

    /// Returns true if this `Kind` prints plain `Body`.
    #[must_use]
    pub const fn is_plain_body(&self) -> bool {
        self.is_plain() && self.is_body()
    }

    /// Returns true if this `Kind` prints color `Body`.
    #[must_use]
    pub const fn is_color_body(&self) -> bool {
        self.is_color() && self.is_body()
    }

    /// Returns the `Parts` contained within this `Kind`.
    #[must_use]
    pub const fn into_parts(&self) -> Parts {
        match *self {
            Self::Plain(parts) | Self::Color(parts) => parts,
        }
    }

    /// Swaps the current `Parts` variant for the `new_parts` variant.
    pub fn swap_parts(&mut self, new_parts: Parts) {
        *self = match self {
            Self::Plain(_) => Self::Plain(new_parts),
            Self::Color(_) => Self::Color(new_parts),
        }
    }

    /// Changes the `Kind` variant to `None`.
    pub fn to_none(&mut self) {
        *self = match self {
            Self::Plain(_) => Self::Plain(Parts::None),
            Self::Color(_) => Self::Color(Parts::None),
        }
    }

    /// Changes the `Kind` variant to `All`.
    pub fn to_all(&mut self) {
        *self = match self {
            Self::Plain(_) => Self::Plain(Parts::All),
            Self::Color(_) => Self::Color(Parts::All),
        }
    }

    /// Changes the `Kind` variant to `Plain`.
    pub fn to_plain(&mut self) {
        if let Self::Color(parts) = self {
            *self = Self::Plain(*parts);
        }
    }

    /// Changes the `Kind` variant to `Color`.
    pub fn to_color(&mut self) {
        if let Self::Plain(parts) = self {
            *self = Self::Color(*parts);
        }
    }
}

/// The output style settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Style {
    pub req: Kind,
    pub res: Kind,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            req: Kind::Color(Parts::None),
            res: Kind::Color(Parts::All)
        }
    }
}

impl WriteCliError for Style {}

impl Style {
    /// Sets the `Style` for requests and responses based upon a
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
    #[allow(clippy::similar_names)]
    pub fn from_format_str(&mut self, format_str: &str) {
        const fn is_good_char(c: char) -> bool {
            matches!(c, 'R' | 'H' | 'B' | 's' | 'h' | 'b' | '*')
        }

        let mut req_num = 0;
        let mut res_num = 0;

        for c in format_str.chars().filter(|&c| is_good_char(c)) {
            match c {
                // "Verbose" style.
                '*' => {
                    self.req.swap_parts(Parts::All);
                    self.res.swap_parts(Parts::All);
                    return;
                },
                // Uppercase letters.
                _ if 97 > c as u32 => req_num += c as u32,
                // Lowercase letters.
                _ => res_num += c as u32,
            }
        }

        match req_num {
            0 => self.req.swap_parts(Parts::None),
            66 => self.req.swap_parts(Parts::Body),
            72 => self.req.swap_parts(Parts::Hdrs),
            82 => self.req.swap_parts(Parts::Line),
            138 => self.req.swap_parts(Parts::HdrsBody),
            148 => self.req.swap_parts(Parts::LineBody),
            154 => self.req.swap_parts(Parts::LineHdrs),
            220 => self.req.swap_parts(Parts::All),
            _ => unreachable!(),
        }

        match res_num {
            0 => self.res.swap_parts(Parts::None),
            98 => self.res.swap_parts(Parts::Body),
            104 => self.res.swap_parts(Parts::Hdrs),
            115 => self.res.swap_parts(Parts::Line),
            202 => self.res.swap_parts(Parts::HdrsBody),
            213 => self.res.swap_parts(Parts::LineBody),
            219 => self.res.swap_parts(Parts::LineHdrs),
            317 => self.res.swap_parts(Parts::All),
            _ => unreachable!(),
        }
    }

    /// Returns true if this `Style` is the "minimal" style.
    #[must_use]
    pub const fn is_minimal(&self) -> bool {
        self.req.into_parts().is_first_line()
            && !self.req.into_parts().is_headers()
            && !self.req.into_parts().is_body()
            && self.res.into_parts().is_first_line()
            && !self.res.into_parts().is_headers()
            && !self.res.into_parts().is_body()
    }

    /// Returns true if this `Style` is the "verbose" style.
    #[must_use]
    pub const fn is_verbose(&self) -> bool {
        self.req.into_parts().is_all()
            && self.res.into_parts().is_all()
    }

    /// Changes the `Kind` variant to `None`.
    pub fn to_none(&mut self) {
        self.req.to_none();
        self.res.to_none();
    }

    /// Changes the `Kind` variant to `All`.
    pub fn to_all(&mut self) {
        self.req.to_all();
        self.res.to_all();
    }

    /// Changes the `Kind` variant to `Plain`.
    pub fn to_plain(&mut self) {
        self.req.to_plain();
        self.res.to_plain();
    }

    /// Changes the `Kind` variant to `Color`.
    pub fn to_color(&mut self) {
        self.req.to_color();
        self.res.to_color();
    }
}

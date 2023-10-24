macro_rules! impl_header_names {
    ($( $name:ident: $text:expr, $bytes:expr; )+) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub enum HeaderName {
            $( $name, )+
            Unknown(String),
        }

        impl fmt::Display for HeaderName {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl From<&[u8]> for HeaderName {
            fn from(buf: &[u8]) -> Self {
                let buf = buf.to_ascii_lowercase();

                match &buf[..] {
                    $( $bytes => Self::$name, )+
                    unk => Self::Unknown(
                        String::from_utf8_lossy(unk).to_string()
                    ),
                }
            }
        }

        impl HeaderName {
            pub fn as_str(&self) -> Cow<'_, str> {
                match self {
                    $( Self::$name => $text.into(), )+
                    Self::Unknown(ref hdr) => hdr.into(),
                }
            }
        }
    };
}

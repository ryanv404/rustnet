use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::{trim_whitespace_bytes, NetError, NetResult};

/// Header name.
/// Non-standard headers are confirmed to only contain valid UTF-8 bytes.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct HeaderName {
    /// Abstraction over standard and non-standard names.
    pub inner: HdrRepr,
}

impl From<StdHeader> for HeaderName {
    fn from(std: StdHeader) -> Self {
        Self {
            inner: HdrRepr::Std(std),
        }
    }
}

impl FromStr for HeaderName {
    type Err = NetError;

    /// Attempts to convert a string slice into a `HeaderName` returning an error
    /// if the header name contains any bytes that are not valid UTF-8.
    fn from_str(s: &str) -> NetResult<Self> {
        let name = Self {
            inner: HdrRepr::try_from(s.as_bytes())?,
        };

        Ok(name)
    }
}

impl TryFrom<&[u8]> for HeaderName {
    type Error = NetError;

    /// Attempts to convert a byte slice into a `HeaderName` returning an error
    /// if the header name contains any bytes that are not valid UTF-8.
    fn try_from(b: &[u8]) -> NetResult<Self> {
        let name = Self {
            inner: HdrRepr::try_from(b)?,
        };

        Ok(name)
    }
}

impl HeaderName {
    /// Creates a new `HeaderName` from a `HdrRepr` representation.
    #[must_use]
    pub const fn new(inner: HdrRepr) -> Self {
        Self { inner }
    }

    /// Returns the header name as a byte slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }

    /// Returns the header name as a title case string.
    #[must_use]
    pub fn to_titlecase(&self) -> String {
        if self.inner.is_empty() {
            return String::new();
        }

        let bytes = self.inner.as_bytes();
        let mut title = String::with_capacity(bytes.len());

        let parts = bytes.split(|&b| b == b'-');

        for (i, part) in parts.enumerate() {
            // Ensure each part has at least one element.
            if part.is_empty() {
                continue;
            }

            // Prepend every part but the first with a hyphen.
            if i > 0 {
                title.push('-');
            }

            // Make the first letter of each part uppercase.
            title.push(part[0].to_ascii_uppercase() as char);

            if part.len() > 1 {
                // Leave the rest of the slice as is.
                title.push_str(str::from_utf8(&part[1..]).unwrap());
            }
        }

        title
    }

    /// Returns the header name as a string slice.
    #[must_use]
    pub fn as_str(&mut self) -> &str {
        str::from_utf8(self.as_bytes()).map_or("", |s| s)
    }
}

impl Display for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.to_titlecase())
    }
}

/// Header name representation.
/// Non-standard headers are confirmed to only contain valid UTF-8 bytes.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum HdrRepr {
    Std(StdHeader),
    Custom(Vec<u8>),
}

impl TryFrom<&[u8]> for HdrRepr {
    type Error = NetError;

    /// Attempts to convert a byte slice into a `HdrRepr` returning an error
    /// if the header name contains any bytes that are not UTF-8.
    fn try_from(b: &[u8]) -> NetResult<Self> {
        match StdHeader::from_bytes(b) {
            Some(std) => Ok(Self::Std(std)),
            None if str::from_utf8(b).is_ok() => Ok(Self::Custom(b.to_ascii_lowercase())),
            None => Err(NetError::NonUtf8Header),
        }
    }
}

impl HdrRepr {
    /// Returns a byte slice representing the header name.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Std(std) => std.as_bytes(),
            Self::Custom(ref hdr_vec) => hdr_vec.as_slice(),
        }
    }

    /// Returns whether the byte representation of the header name has a
    /// length of zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Std(std) => std.as_bytes().is_empty(),
            Self::Custom(ref hdr_vec) => hdr_vec.is_empty(),
        }
    }
}

macro_rules! impl_header_names {
    ($( $bytes:literal => $constant:ident, $variant:ident; )+) => {
        // Standard header names.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
        pub enum StdHeader {
            $( $variant, )+
        }

        pub mod header_names {
            use super::{HdrRepr, HeaderName, StdHeader};

            $(
                // Constants representing all of the standard header names.
                pub const $constant: HeaderName = HeaderName {
                    inner: HdrRepr::Std(StdHeader::$variant)
                };
            )+
        }

        impl Display for StdHeader {
            fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                write!(f, "{}", self.as_str())
            }
        }

        impl StdHeader {
            /// Attempts to convert a byte slice into a `StdHeader`, returning `None`
            /// if it cannot do so.
            #[must_use]
            pub fn from_bytes(input: &[u8]) -> Option<Self> {
                let lowercase = trim_whitespace_bytes(input).to_ascii_lowercase();

                match lowercase.as_slice() {
                    $( $bytes => Some(Self::$variant), )+
                    _ => None,
                }
            }

            /// Returns the header name as a byte slice.
            #[must_use]
            pub const fn as_bytes(&self) -> &'static [u8] {
                match *self {
                    $( Self::$variant => $bytes, )+
                }
            }

            /// Returns the header name as a string slice.
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                // SAFETY: We know that the bytes are valid UTF-8 since we provided them.
                unsafe {
                    str::from_utf8_unchecked(self.as_bytes())
                }
            }
        }

        #[cfg(test)]
        const TEST_HEADERS: &'static [(StdHeader, &'static [u8])] = &[
            $( (StdHeader::$variant, $bytes), )+
        ];

        #[cfg(test)]
        #[test]
        fn test_parse_std_headers() {
            // Lowercase bytes test.
            TEST_HEADERS.iter().for_each(|&(std, bytes)| {
                let std_hdr = HeaderName::from(std);
                let parsed_hdr = HeaderName::try_from(bytes).unwrap();

                assert_eq!(std_hdr, parsed_hdr);
            });

            // Uppercase bytes test.
            TEST_HEADERS.iter().for_each(|&(std, bytes)| {
                let std_hdr = HeaderName::from(std);
                let parsed_hdr = HeaderName::try_from(
                    bytes.to_ascii_uppercase().as_slice()
                ).unwrap();

                assert_eq!(std_hdr, parsed_hdr);
            });
        }
    };
}

impl_header_names! {
    b"accept" => ACCEPT, Accept;
    b"accept-charset" => ACCEPT_CHARSET, AcceptCharset;
    b"accept-datetime" => ACCEPT_DATETIME, AcceptDatetime;
    b"accept-encoding" => ACCEPT_ENCODING, AcceptEncoding;
    b"accept-language" => ACCEPT_LANGUAGE, AcceptLanguage;
    b"accept-patch" => ACCEPT_PATCH, AcceptPatch;
    b"accept-post" => ACCEPT_POST, AcceptPost;
    b"accept-ranges" => ACCEPT_RANGES, AcceptRanges;
    b"access-control-allow-credentials" => ACCESS_CONTROL_ALLOW_CREDENTIALS,
        AccessControlAllowCredentials;
    b"access-control-allow-headers" => ACCESS_CONTROL_ALLOW_HEADERS,
        AccessControlAllowHeaders;
    b"access-control-allow-methods" => ACCESS_CONTROL_ALLOW_METHODS,
        AccessControlAllowMethods;
    b"access-control-allow-origin" => ACCESS_CONTROL_ALLOW_ORIGIN,
        AccessControlAllowOrigin;
    b"access-control-expose-headers" => ACCESS_CONTROL_EXPOSE_HEADERS,
        AccessControlExposeHeaders;
    b"access-control-max-age" => ACCESS_CONTROL_MAX_AGE, AccessControlMaxAge;
    b"access-control-request-headers" => ACCESS_CONTROL_REQUEST_HEADERS,
        AccessControlRequestHeaders;
    b"access-control-request-method" => ACCESS_CONTROL_REQUEST_METHOD,
        AccessControlRequestMethod;
    b"age" => AGE, Age;
    b"allow" => ALLOW, Allow;
    b"alt-svc" => ALT_SVC, AltSvc;
    b"authorization" => AUTHORIZATION, Authorization;
    b"cache-control" => CACHE_CONTROL, CacheControl;
    b"cache-status" => CACHE_STATUS, CacheStatus;
    b"cdn-cache-control" => CDN_CACHE_CONTROL, CdnCacheControl;
    b"clear-site-data" => CLEAR_SITE_DATA, ClearSiteData;
    b"connection" => CONNECTION, Connection;
    b"content-disposition" => CONTENT_DISPOSITION, ContentDisposition;
    b"content-encoding" => CONTENT_ENCODING, ContentEncoding;
    b"content-language" => CONTENT_LANGUAGE, ContentLanguage;
    b"content-length" => CONTENT_LENGTH, ContentLength;
    b"content-location" => CONTENT_LOCATION, ContentLocation;
    b"content-range" => CONTENT_RANGE, ContentRange;
    b"content-security-policy" => CONTENT_SECURITY_POLICY,
        ContentSecurityPolicy;
    b"content-security-policy-report-only" =>
        CONTENT_SECURITY_POLICY_REPORT_ONLY, ContentSecurityPolicyReportOnly;
    b"content-type" => CONTENT_TYPE, ContentType;
    b"cookie" => COOKIE, Cookie;
    b"cross-origin-embedder-policy" => CROSS_ORIGIN_EMBEDDER_POLICY,
        CrossOriginEmbedderPolicy;
    b"cross-origin-opener-policy" => CROSS_ORIGIN_OPENER_POLICY,
        CrossOriginOpenerPolicy;
    b"cross-origin-resource-policy" => CROSS_ORIGIN_RESOURCE_POLICY,
        CrossOriginResourcePolicy;
    b"date" => DATE, Date;
    b"delta-base" => DELTA_BASE, DeltaBase;
    b"device-memory" => DEVICE_MEMORY, DeviceMemory;
    b"digest" => DIGEST, Digest;
    b"dnt" => DNT, Dnt;
    b"etag" => ETAG, Etag;
    b"expect" => EXPECT, Expect;
    b"expires" => EXPIRES, Expires;
    b"forwarded" => FORWARDED, Forwarded;
    b"from" => FROM, From;
    b"host" => HOST, Host;
    b"if-match" => IF_MATCH, IfMatch;
    b"if-modified-since" => IF_MODIFIED_SINCE, IfModifiedSince;
    b"if-none-match" => IF_NONE_MATCH, IfNoneMatch;
    b"if-range" => IF_RANGE, IfRange;
    b"if-unmodified-since" => IF_UNMODIFIED_SINCE, IfUnmodifiedSince;
    b"http2-settings" => HTTP2_SETTINGS, Http2Settings;
    b"keep-alive" => KEEP_ALIVE, KeepAlive;
    b"last-modified" => LAST_MODIFIED, LastModified;
    b"link" => LINK, Link;
    b"location" => LOCATION, Location;
    b"max-forwards" => MAX_FORWARDS, MaxForwards;
    b"origin" => ORIGIN, Origin;
    b"permissions-policy" => PERMISSIONS_POLICY, PermissionsPolicy;
    b"pragma" => PRAGMA, Pragma;
    b"prefer" => PREFER, Prefer;
    b"proxy-authenticate" => PROXY_AUTHENTICATE, ProxyAuthenticate;
    b"proxy-authorization" => PROXY_AUTHORIZATION, ProxyAuthorization;
    b"public-key-pins" => PUBLIC_KEY_PINS, PublicKeyPins;
    b"public-key-pins-report-only" => PUBLIC_KEY_PINS_REPORT_ONLY,
        PublicKeyPinsReportOnly;
    b"purpose" => PURPOSE, Purpose;
    b"range" => RANGE, Range;
    b"referer" => REFERER, Referer;
    b"referrer-policy" => REFERRER_POLICY, ReferrerPolicy;
    b"refresh" => REFRESH, Refresh;
    b"retry-after" => RETRY_AFTER, RetryAfter;
    b"sec-ch-ua" => SEC_CH_UA, SecChUa;
    b"sec-ch-ua-mobile" => SEC_CH_UA_MOBILE, SecChUaMobile;
    b"sec-ch-ua-platform" => SEC_CH_UA_PLATFORM, SecChUaPlatform;
    b"save-data" => SAVE_DATA, SaveData;
    b"sec-fetch-dest" => SEC_FETCH_DEST, SecFetchDest;
    b"sec-fetch-mode" => SEC_FETCH_MODE, SecFetchMode;
    b"sec-fetch-site" => SEC_FETCH_SITE, SecFetchSite;
    b"sec-fetch-user" => SEC_FETCH_USER, SecFetchUser;
    b"sec-gpc" => SEC_GPC, SecGpc;
    b"sec-websocket-accept" => SEC_WEBSOCKET_ACCEPT, SecWebsocketAccept;
    b"sec-websocket-extensions" => SEC_WEBSOCKET_EXTENSIONS,
        SecWebsocketExtensions;
    b"sec-websocket-key" => SEC_WEBSOCKET_KEY, SecWebsocketKey;
    b"sec-websocket-protocol" => SEC_WEBSOCKET_PROTOCOL, SecWebsocketProtocol;
    b"sec-websocket-version" => SEC_WEBSOCKET_VERSION, SecWebsocketVersion;
    b"server" => SERVER, Server;
    b"server-timing" => SERVER_TIMING, ServerTiming;
    b"set-cookie" => SET_COOKIE, SetCookie;
    b"sourcemap" => SOURCEMAP, Sourcemap;
    b"strict-transport-security" => STRICT_TRANSPORT_SECURITY,
        StrictTransportSecurity;
    b"te" => TE, Te;
    b"timing-allow-origin" => TIMING_ALLOW_ORIGIN, TimingAllowOrigin;
    b"trailer" => TRAILER, Trailer;
    b"transfer-encoding" => TRANSFER_ENCODING, TransferEncoding;
    b"user-agent" => USER_AGENT, UserAgent;
    b"upgrade" => UPGRADE, Upgrade;
    b"upgrade-insecure-requests" => UPGRADE_INSECURE_REQUESTS,
        UpgradeInsecureRequests;
    b"vary" => VARY, Vary;
    b"via" => VIA, Via;
    b"want-digest" => WANT_DIGEST, WantDigest;
    b"warning" => WARNING, Warning;
    b"www-authenticate" => WWW_AUTHENTICATE, WwwAuthenticate;
    b"x-content-type-options" => X_CONTENT_TYPE_OPTIONS, XContentTypeOptions;
    b"x-dns-prefetch-control" => X_DNS_PREFETCH_CONTROL, XDnsPrefetchControl;
    b"x-forwarded-for" => X_FORWARDED_FOR, XForwardedFor;
    b"x-forwarded-host" => X_FORWARDED_HOST, XForwardedHost;
    b"x-forwarded-proto" => X_FORWARDED_PROTO, XForwardedProto;
    b"x-frame-options" => X_FRAME_OPTIONS, XFrameOptions;
    b"x-powered-by" => X_POWERED_BY, XPoweredBy;
    b"x-request-id" => X_REQUEST_ID, XRequestId;
    b"x-robots-tag" => X_ROBOTS_TAG, XRobotsTag;
    b"x-ua-compatible" => X_UA_COMPATIBLE, XUaCompatible;
    b"x-xss-protection" => X_XSS_PROTECTION, XXssProtection;
}

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::util::trim_whitespace_bytes;
use crate::{NetError, NetParseError, NetResult};

/// Header field name.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderName {
    pub inner: HeaderKind,
}

impl Display for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.to_titlecase())
    }
}

impl Debug for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(&self.to_titlecase(), f)
    }
}

impl From<StandardHeader> for HeaderName {
    fn from(std: StandardHeader) -> Self {
        let inner = HeaderKind::Standard(std);
        Self { inner }
    }
}

impl FromStr for HeaderName {
    type Err = NetError;

    fn from_str(s: &str) -> NetResult<Self> {
        let inner = StandardHeader::from_bytes(s.as_bytes()).map_or_else(
            || HeaderKind::Custom(Vec::from(s.trim())),
            HeaderKind::Standard,
        );

        Ok(Self { inner })
    }
}

impl From<&str> for HeaderName {
    fn from(s: &str) -> Self {
        let s = s.trim();
        let bytes = s.as_bytes();

        Self {
            inner: StandardHeader::from_bytes(bytes).map_or_else(
                || HeaderKind::Custom(Vec::from(s)),
                HeaderKind::Standard)
        }
    }
}

impl TryFrom<&[u8]> for HeaderName {
    type Error = NetError;

    fn try_from(b: &[u8]) -> NetResult<Self> {
        let inner = HeaderKind::try_from(b)?;
        Ok(Self { inner })
    }
}

impl HeaderName {
    /// Returns a standard `HeaderName` with an inner `HeaderKind::Standard`
    /// from the given string slice, if possible.
    #[must_use]
    pub fn standard(name: &str) -> Option<Self> {
        StandardHeader::from_bytes(name.as_bytes()).map(|hdr| Self {
            inner: HeaderKind::Standard(hdr),
        })
    }

    /// Returns a custom `HeaderName` with an inner `HeaderKind::Custom` from
    /// the given string slice.
    #[must_use]
    pub fn custom(name: &str) -> Self {
        Self {
            inner: HeaderKind::Custom(Vec::from(name)),
        }
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

        bytes
            .split(|&b| b == b'-')
            .filter(|&part| !part.is_empty())
            .for_each(|part| {
                if let Some((first, rest)) = part.split_first() {
                    // Prepend every part but the first with a hyphen.
                    if !title.is_empty() {
                        title.push('-');
                    }

                    title.push(first.to_ascii_uppercase() as char);

                    if !rest.is_empty() {
                        title.push_str(&String::from_utf8_lossy(rest));
                    }
                }
            });

        title
    }
}

/// Header name representation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderKind {
    Standard(StandardHeader),
    Custom(Vec<u8>),
}

impl TryFrom<&[u8]> for HeaderKind {
    type Error = NetError;

    fn try_from(b: &[u8]) -> NetResult<Self> {
        match StandardHeader::from_bytes(b) {
            Some(std) => Ok(Self::Standard(std)),
            None if str::from_utf8(b).is_ok() => {
                Ok(Self::Custom(b.to_ascii_lowercase()))
            },
            None => Err(NetError::Parse(NetParseError::Header)),
        }
    }
}

impl HeaderKind {
    /// Returns the header field name as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Standard(std) => std.as_bytes(),
            Self::Custom(ref buf) => buf.as_slice(),
        }
    }

    /// Returns whether the byte representation of the header name has a
    /// length of zero.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Standard(std) => std.as_bytes().is_empty(),
            Self::Custom(ref buf) => buf.is_empty(),
        }
    }
}

macro_rules! impl_standard_headers {
    ($( $bytes:literal => $constant:ident, $variant:ident; )+) => {
        // Standard header names.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
        pub enum StandardHeader {
            $( $variant, )+
        }

        pub mod header_consts {
            use super::{HeaderKind, HeaderName, StandardHeader};
            $(
                // Constants representing all of the standard header field names.
                pub const $constant: HeaderName = HeaderName {
                    inner: HeaderKind::Standard(StandardHeader::$variant)
                };
            )+
        }

        impl Display for StandardHeader {
            fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                write!(f, "{}", self.as_str())
            }
        }

        impl StandardHeader {
            /// Parses a bytes slice into a `StandardHeader` if possible.
            #[must_use]
            pub fn from_bytes(input: &[u8]) -> Option<Self> {
                let lowercase = trim_whitespace_bytes(input)
                    .to_ascii_lowercase();

                match lowercase.as_slice() {
                    $( $bytes => Some(Self::$variant), )+
                    _ => None,
                }
            }

            /// Returns a bytes slice representation of the `StandardHeader`.
            #[must_use]
            pub const fn as_bytes(&self) -> &'static [u8] {
                match *self {
                    $( Self::$variant => $bytes, )+
                }
            }

            /// Returns a string slice representation of the `StandardHeader`.
            #[must_use]
            pub fn as_str(&self) -> &'static str {
                // NOTE: The standard headers below are all UTF-8 compatible.
                str::from_utf8(self.as_bytes()).unwrap()
            }
        }

        // A collection of all `StandardHeader` values that is used during
        // testing.
        #[cfg(test)]
        pub const STANDARD_HEADERS: &'static [(StandardHeader, &'static [u8])] = &[
            $( (StandardHeader::$variant, $bytes), )+
        ];
    };
}

impl_standard_headers! {
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
    b"x-more-info" => X_MORE_INFO, XMoreInfo;
    b"x-powered-by" => X_POWERED_BY, XPoweredBy;
    b"x-request-id" => X_REQUEST_ID, XRequestId;
    b"x-robots-tag" => X_ROBOTS_TAG, XRobotsTag;
    b"x-ua-compatible" => X_UA_COMPATIBLE, XUaCompatible;
    b"x-xss-protection" => X_XSS_PROTECTION, XXssProtection;
}
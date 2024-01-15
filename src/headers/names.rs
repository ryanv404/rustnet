use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::{NetParseError, utils};

/// An HTTP header name.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeaderName {
    pub inner: HeaderNameInner,
}

impl Display for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for HeaderName {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for HeaderName {
    fn from(name: &str) -> Self {
        Self { inner: HeaderNameInner::from(name) }
    }
}

impl TryFrom<&[u8]> for HeaderName {
    type Error = NetParseError;

    fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(name)
            .map_err(|_| NetParseError::Header)
            .map(Into::into)
    }
}

impl HeaderName {
    /// Returns the `HeaderName` as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    /// Returns the `HeaderName` as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

/// A representation of header names as either standard or custom.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderNameInner {
    Standard(StandardHeaderName),
    Custom(String),
}

impl Display for HeaderNameInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for HeaderNameInner {
    fn from(inner: &str) -> Self {
        if let Ok(std) = StandardHeaderName::from_str(inner) {
            Self::Standard(std)
        } else {
            // Store custom header names in titlecase.
            let inner = utils::to_titlecase(inner.trim());
            Self::Custom(inner)
        }
    }
}

impl TryFrom<&[u8]> for HeaderNameInner {
    type Error = NetParseError;

    fn try_from(inner: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(inner)
            .map_err(|_| NetParseError::Header)
            .map(Into::into)
    }
}

impl HeaderNameInner {
    /// Returns this `HeaderNameInner` variant as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Standard(std) => std.as_str(),
            Self::Custom(ref custom) => custom.as_str(),
        }
    }

    /// Returns this `HeaderNameInner` variant as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Standard(std) => std.as_bytes(),
            Self::Custom(ref custom) => custom.as_bytes(),
        }
    }
}

macro_rules! impl_standard_header_names {
    ($( $text:literal, $constant:ident, $variant:ident; )+) => {
        /// Standard HTTP header names.
        #[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
        pub enum StandardHeaderName {
            $( $variant, )+
        }

        impl Display for StandardHeaderName {
            fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
                write!(f, "{}", self.as_str())
            }
        }

        impl FromStr for StandardHeaderName {
            type Err = NetParseError;

            fn from_str(name: &str) -> Result<Self, Self::Err> {
                let name = utils::to_titlecase(name.trim());

                match name.as_str() {
                    $( $text => Ok(Self::$variant), )+
                    _ => Err(NetParseError::Header),
                }
            }
        }

        impl TryFrom<&[u8]> for StandardHeaderName {
            type Error = NetParseError;

            fn try_from(name: &[u8]) -> Result<Self, Self::Error> {
                str::from_utf8(name)
                    .map_err(|_| NetParseError::Header)
                    .and_then(Self::from_str)
            }
        }

        impl StandardHeaderName {
            /// Returns the `StandardHeaderName` as a string slice.
            #[must_use]
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $( Self::$variant => $text, )+
                }
            }

            /// Returns the `StandardHeaderName` as a bytes slice.
            #[must_use]
            pub const fn as_bytes(&self) -> &'static [u8] {
                self.as_str().as_bytes()
            }
        }

        /// `HeaderName` constants for each `StandardHeaderName` variant.
        $(
            pub const $constant: HeaderName = HeaderName {
                inner: HeaderNameInner::Standard(
                    StandardHeaderName::$variant
                )
            };
        )+

        /// An array of all `StandardHeaderName` variants and their string
        /// slice representations.
        #[cfg(test)]
        pub const STD_HEADER_NAMES: &'static [
            (StandardHeaderName, &'static str)
        ] = &[
            $( (StandardHeaderName::$variant, $text), )+
        ];
    };
}

impl_standard_header_names! {
    "Accept", ACCEPT, Accept;
    "Accept-Charset", ACCEPT_CHARSET, AcceptCharset;
    "Accept-Datetime", ACCEPT_DATETIME, AcceptDatetime;
    "Accept-Encoding", ACCEPT_ENCODING, AcceptEncoding;
    "Accept-Language", ACCEPT_LANGUAGE, AcceptLanguage;
    "Accept-Patch", ACCEPT_PATCH, AcceptPatch;
    "Accept-Post", ACCEPT_POST, AcceptPost;
    "Accept-Ranges", ACCEPT_RANGES, AcceptRanges;
    "Access-Control-Allow-Credentials", ACCESS_CONTROL_ALLOW_CREDENTIALS,
        AccessControlAllowCredentials;
    "Access-Control-Allow-Headers", ACCESS_CONTROL_ALLOW_HEADERS,
        AccessControlAllowHeaders;
    "Access-Control-Allow-Methods", ACCESS_CONTROL_ALLOW_METHODS,
        AccessControlAllowMethods;
    "Access-Control-Allow-Origin", ACCESS_CONTROL_ALLOW_ORIGIN,
        AccessControlAllowOrigin;
    "Access-Control-Expose-Headers", ACCESS_CONTROL_EXPOSE_HEADERS,
        AccessControlExposeHeaders;
    "Access-Control-Max-Age", ACCESS_CONTROL_MAX_AGE, AccessControlMaxAge;
    "Access-Control-Request-Headers", ACCESS_CONTROL_REQUEST_HEADERS,
        AccessControlRequestHeaders;
    "Access-Control-Request-Method", ACCESS_CONTROL_REQUEST_METHOD,
        AccessControlRequestMethod;
    "Age", AGE, Age;
    "Allow", ALLOW, Allow;
    "Alt-Svc", ALT_SVC, AltSvc;
    "Authorization", AUTHORIZATION, Authorization;
    "Cache-Control", CACHE_CONTROL, CacheControl;
    "Cache-Status", CACHE_STATUS, CacheStatus;
    "Cdn-Cache-Control", CDN_CACHE_CONTROL, CdnCacheControl;
    "Clear-Site-Data", CLEAR_SITE_DATA, ClearSiteData;
    "Connection", CONNECTION, Connection;
    "Content-Disposition", CONTENT_DISPOSITION, ContentDisposition;
    "Content-Encoding", CONTENT_ENCODING, ContentEncoding;
    "Content-Language", CONTENT_LANGUAGE, ContentLanguage;
    "Content-Length", CONTENT_LENGTH, ContentLength;
    "Content-Location", CONTENT_LOCATION, ContentLocation;
    "Content-Range", CONTENT_RANGE, ContentRange;
    "Content-Security-Policy", CONTENT_SECURITY_POLICY, ContentSecurityPolicy;
    "Content-Security-Policy-Report-Only", CONTENT_SECURITY_POLICY_REPORT_ONLY,
        ContentSecurityPolicyReportOnly;
    "Content-Type", CONTENT_TYPE, ContentType;
    "Cookie", COOKIE, Cookie;
    "Cross-Origin-Embedder-Policy", CROSS_ORIGIN_EMBEDDER_POLICY,
        CrossOriginEmbedderPolicy;
    "Cross-Origin-Opener-Policy", CROSS_ORIGIN_OPENER_POLICY,
        CrossOriginOpenerPolicy;
    "Cross-Origin-Resource-Policy", CROSS_ORIGIN_RESOURCE_POLICY,
        CrossOriginResourcePolicy;
    "Date", DATE, Date;
    "Delta-Base", DELTA_BASE, DeltaBase;
    "Device-Memory", DEVICE_MEMORY, DeviceMemory;
    "Digest", DIGEST, Digest;
    "Dnt", DNT, Dnt;
    "Etag", ETAG, Etag;
    "Expect", EXPECT, Expect;
    "Expires", EXPIRES, Expires;
    "Forwarded", FORWARDED, Forwarded;
    "From", FROM, From;
    "Host", HOST, Host;
    "If-Match", IF_MATCH, IfMatch;
    "If-Modified-Since", IF_MODIFIED_SINCE, IfModifiedSince;
    "If-None-Match", IF_NONE_MATCH, IfNoneMatch;
    "If-Range", IF_RANGE, IfRange;
    "If-Unmodified-Since", IF_UNMODIFIED_SINCE, IfUnmodifiedSince;
    "Http2-Settings", HTTP2_SETTINGS, Http2Settings;
    "Keep-Alive", KEEP_ALIVE, KeepAlive;
    "Last-Modified", LAST_MODIFIED, LastModified;
    "Link", LINK, Link;
    "Location", LOCATION, Location;
    "Max-Forwards", MAX_FORWARDS, MaxForwards;
    "Origin", ORIGIN, Origin;
    "Permissions-Policy", PERMISSIONS_POLICY, PermissionsPolicy;
    "Pragma", PRAGMA, Pragma;
    "Prefer", PREFER, Prefer;
    "Proxy-Authenticate", PROXY_AUTHENTICATE, ProxyAuthenticate;
    "Proxy-Authorization", PROXY_AUTHORIZATION, ProxyAuthorization;
    "Public-Key-Pins", PUBLIC_KEY_PINS, PublicKeyPins;
    "Public-Key-Pins-Report-Only", PUBLIC_KEY_PINS_REPORT_ONLY,
        PublicKeyPinsReportOnly;
    "Purpose", PURPOSE, Purpose;
    "Range", RANGE, Range;
    "Referer", REFERER, Referer;
    "Referrer-Policy", REFERRER_POLICY, ReferrerPolicy;
    "Refresh", REFRESH, Refresh;
    "Retry-After", RETRY_AFTER, RetryAfter;
    "Sec-Ch-Ua", SEC_CH_UA, SecChUa;
    "Sec-Ch-Ua-Mobile", SEC_CH_UA_MOBILE, SecChUaMobile;
    "Sec-Ch-Ua-Platform", SEC_CH_UA_PLATFORM, SecChUaPlatform;
    "Save-Data", SAVE_DATA, SaveData;
    "Sec-Fetch-Dest", SEC_FETCH_DEST, SecFetchDest;
    "Sec-Fetch-Mode", SEC_FETCH_MODE, SecFetchMode;
    "Sec-Fetch-Site", SEC_FETCH_SITE, SecFetchSite;
    "Sec-Fetch-User", SEC_FETCH_USER, SecFetchUser;
    "Sec-Gpc", SEC_GPC, SecGpc;
    "Sec-Websocket-Accept", SEC_WEBSOCKET_ACCEPT, SecWebsocketAccept;
    "Sec-Websocket-Extensions", SEC_WEBSOCKET_EXTENSIONS,
        SecWebsocketExtensions;
    "Sec-Websocket-Key", SEC_WEBSOCKET_KEY, SecWebsocketKey;
    "Sec-Websocket-Protocol", SEC_WEBSOCKET_PROTOCOL, SecWebsocketProtocol;
    "Sec-Websocket-Version", SEC_WEBSOCKET_VERSION, SecWebsocketVersion;
    "Server", SERVER, Server;
    "Server-Timing", SERVER_TIMING, ServerTiming;
    "Set-Cookie", SET_COOKIE, SetCookie;
    "Sourcemap", SOURCEMAP, Sourcemap;
    "Strict-Transport-Security", STRICT_TRANSPORT_SECURITY,
        StrictTransportSecurity;
    "Te", TE, Te;
    "Timing-Allow-Origin", TIMING_ALLOW_ORIGIN, TimingAllowOrigin;
    "Trailer", TRAILER, Trailer;
    "Transfer-Encoding", TRANSFER_ENCODING, TransferEncoding;
    "User-Agent", USER_AGENT, UserAgent;
    "Upgrade", UPGRADE, Upgrade;
    "Upgrade-Insecure-Requests", UPGRADE_INSECURE_REQUESTS,
        UpgradeInsecureRequests;
    "Vary", VARY, Vary;
    "Via", VIA, Via;
    "Want-Digest", WANT_DIGEST, WantDigest;
    "Warning", WARNING, Warning;
    "Www-Authenticate", WWW_AUTHENTICATE, WwwAuthenticate;
    "X-Content-Type-Options", X_CONTENT_TYPE_OPTIONS, XContentTypeOptions;
    "X-Dns-Prefetch-Control", X_DNS_PREFETCH_CONTROL, XDnsPrefetchControl;
    "X-Forwarded-For", X_FORWARDED_FOR, XForwardedFor;
    "X-Forwarded-Host", X_FORWARDED_HOST, XForwardedHost;
    "X-Forwarded-Proto", X_FORWARDED_PROTO, XForwardedProto;
    "X-Frame-Options", X_FRAME_OPTIONS, XFrameOptions;
    "X-More-Info", X_MORE_INFO, XMoreInfo;
    "X-Powered-By", X_POWERED_BY, XPoweredBy;
    "X-Request-Id", X_REQUEST_ID, XRequestId;
    "X-Robots-Tag", X_ROBOTS_TAG, XRobotsTag;
    "X-Ua-Compatible", X_UA_COMPATIBLE, XUaCompatible;
    "X-Xss-Protection", X_XSS_PROTECTION, XXssProtection;
}

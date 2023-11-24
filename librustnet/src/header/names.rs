use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str;

use crate::{trim_whitespace_bytes, NetError, NetResult, ParseErrorKind};

/// Header field name.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl From<&str> for HeaderName {
    fn from(s: &str) -> Self {
        Self { inner: s.into() }
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
    /// Creates a new `HeaderName` from the given `HeaderKind`.
    #[must_use]
    pub const fn new(inner: HeaderKind) -> Self {
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

        bytes.split(|&b| b == b'-')
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
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum HeaderKind {
    Standard(StandardHeader),
    Custom(Vec<u8>),
}

impl From<&str> for HeaderKind {
    fn from(s: &str) -> Self {
        StandardHeader::from_bytes(s.as_bytes()).map_or_else(
            || Self::Custom(Vec::from(s)),
            |header| Self::Standard(header)
        )
    }
}

impl TryFrom<&[u8]> for HeaderKind {
    type Error = NetError;

    fn try_from(b: &[u8]) -> NetResult<Self> {
        match StandardHeader::from_bytes(b) {
            Some(std) => Ok(Self::Standard(std)),
            None if str::from_utf8(b).is_ok() => {
                Ok(Self::Custom(b.to_ascii_lowercase()))
            },
            None => Err(ParseErrorKind::Header.into()),
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

macro_rules! impl_header_names {
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

        #[cfg(test)]
        pub const TEST_HEADERS: &'static [(StandardHeader, &'static [u8])] = &[
            $( (StandardHeader::$variant, $bytes), )+
        ];
    };
}

impl_header_names! {
    // Accept = [ ( media-range [ weight ] ) *( OWS "," OWS ( media-range [ weight ] ) ) ]
    b"accept" => ACCEPT, Accept;
    // Accept-Charset = [ ( ( token / "*" ) [ weight ] ) *( OWS "," OWS ( (
    // token / "*" ) [ weight ] ) ) ]
    b"accept-charset" => ACCEPT_CHARSET, AcceptCharset;
    b"accept-datetime" => ACCEPT_DATETIME, AcceptDatetime;
    // Accept-Encoding = [ ( codings [ weight ] ) *( OWS "," OWS ( codings [
    // weight ] ) ) ]
    b"accept-encoding" => ACCEPT_ENCODING, AcceptEncoding;
    // Accept-Language = [ ( language-range [ weight ] ) *( OWS "," OWS (
    // language-range [ weight ] ) ) ]
    b"accept-language" => ACCEPT_LANGUAGE, AcceptLanguage;
    b"accept-patch" => ACCEPT_PATCH, AcceptPatch;
    b"accept-post" => ACCEPT_POST, AcceptPost;
    // Accept-Ranges = acceptable-ranges
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
    // Allow = [ method *( OWS "," OWS method ) ]
    b"allow" => ALLOW, Allow;
    b"alt-svc" => ALT_SVC, AltSvc;
    // Authorization = credentials
    b"authorization" => AUTHORIZATION, Authorization;
    b"cache-control" => CACHE_CONTROL, CacheControl;
    b"cache-status" => CACHE_STATUS, CacheStatus;
    b"cdn-cache-control" => CDN_CACHE_CONTROL, CdnCacheControl;
    b"clear-site-data" => CLEAR_SITE_DATA, ClearSiteData;
    // Connection = [ connection-option *( OWS "," OWS connection-option ) ]
    b"connection" => CONNECTION, Connection;
    b"content-disposition" => CONTENT_DISPOSITION, ContentDisposition;
    // Content-Encoding = [ content-coding *( OWS "," OWS content-coding ) ]
    b"content-encoding" => CONTENT_ENCODING, ContentEncoding;
    // Content-Language = [ language-tag *( OWS "," OWS language-tag ) ]
    b"content-language" => CONTENT_LANGUAGE, ContentLanguage;
    // Content-Length = 1*DIGIT
    b"content-length" => CONTENT_LENGTH, ContentLength;
    // Content-Location = absolute-URI / partial-URI
    b"content-location" => CONTENT_LOCATION, ContentLocation;
    // Content-Range = range-unit SP ( range-resp / unsatisfied-range )
    b"content-range" => CONTENT_RANGE, ContentRange;
    b"content-security-policy" => CONTENT_SECURITY_POLICY,
        ContentSecurityPolicy;
    b"content-security-policy-report-only" =>
        CONTENT_SECURITY_POLICY_REPORT_ONLY, ContentSecurityPolicyReportOnly;
    // Content-Type = media-type
    b"content-type" => CONTENT_TYPE, ContentType;
    b"cookie" => COOKIE, Cookie;
    b"cross-origin-embedder-policy" => CROSS_ORIGIN_EMBEDDER_POLICY,
        CrossOriginEmbedderPolicy;
    b"cross-origin-opener-policy" => CROSS_ORIGIN_OPENER_POLICY,
        CrossOriginOpenerPolicy;
    b"cross-origin-resource-policy" => CROSS_ORIGIN_RESOURCE_POLICY,
        CrossOriginResourcePolicy;
    // Date = IMF-fixdate / obs-date
    b"date" => DATE, Date;
    b"delta-base" => DELTA_BASE, DeltaBase;
    b"device-memory" => DEVICE_MEMORY, DeviceMemory;
    b"digest" => DIGEST, Digest;
    b"dnt" => DNT, Dnt;
    // ETag = entity-tag
    b"etag" => ETAG, Etag;
    // Expect = [ expectation *( OWS "," OWS expectation ) ]
    b"expect" => EXPECT, Expect;
    b"expires" => EXPIRES, Expires;
    b"forwarded" => FORWARDED, Forwarded;
    // From = mailbox
    b"from" => FROM, From;
    // Host = uri-host [ ":" port ]
    b"host" => HOST, Host;
    // If-Match = "*" / [ entity-tag *( OWS "," OWS entity-tag ) ]
    b"if-match" => IF_MATCH, IfMatch;
    // If-Modified-Since = HTTP-date
    b"if-modified-since" => IF_MODIFIED_SINCE, IfModifiedSince;
    // If-None-Match = "*" / [ entity-tag *( OWS "," OWS entity-tag ) ]
    b"if-none-match" => IF_NONE_MATCH, IfNoneMatch;
    // If-Range = entity-tag / HTTP-date
    b"if-range" => IF_RANGE, IfRange;
    // If-Unmodified-Since = HTTP-date
    b"if-unmodified-since" => IF_UNMODIFIED_SINCE, IfUnmodifiedSince;
    b"http2-settings" => HTTP2_SETTINGS, Http2Settings;
    b"keep-alive" => KEEP_ALIVE, KeepAlive;
    // Last-Modified = HTTP-date
    b"last-modified" => LAST_MODIFIED, LastModified;
    b"link" => LINK, Link;
    // Location = URI-reference
    b"location" => LOCATION, Location;
    b"max-forwards" => MAX_FORWARDS, MaxForwards;
    b"origin" => ORIGIN, Origin;
    b"permissions-policy" => PERMISSIONS_POLICY, PermissionsPolicy;
    b"pragma" => PRAGMA, Pragma;
    b"prefer" => PREFER, Prefer;
    // Proxy-Authenticate = [ challenge *( OWS "," OWS challenge ) ]
    b"proxy-authenticate" => PROXY_AUTHENTICATE, ProxyAuthenticate;
    // Proxy-Authorization = credentials
    b"proxy-authorization" => PROXY_AUTHORIZATION, ProxyAuthorization;
    b"public-key-pins" => PUBLIC_KEY_PINS, PublicKeyPins;
    b"public-key-pins-report-only" => PUBLIC_KEY_PINS_REPORT_ONLY,
        PublicKeyPinsReportOnly;
    b"purpose" => PURPOSE, Purpose;
    // Range = ranges-specifier
    b"range" => RANGE, Range;
    // Referer = absolute-URI / partial-URI
    b"referer" => REFERER, Referer;
    b"referrer-policy" => REFERRER_POLICY, ReferrerPolicy;
    b"refresh" => REFRESH, Refresh;
    // Max-Forwards = 1*DIGIT
    // Retry-After = HTTP-date / delay-seconds
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
    // Server = product *( RWS ( product / comment ) )
    b"server" => SERVER, Server;
    b"server-timing" => SERVER_TIMING, ServerTiming;
    b"set-cookie" => SET_COOKIE, SetCookie;
    b"sourcemap" => SOURCEMAP, Sourcemap;
    b"strict-transport-security" => STRICT_TRANSPORT_SECURITY,
        StrictTransportSecurity;
    // TE = [ t-codings *( OWS "," OWS t-codings ) ]
    b"te" => TE, Te;
    b"timing-allow-origin" => TIMING_ALLOW_ORIGIN, TimingAllowOrigin;
    // Trailer = [ field-name *( OWS "," OWS field-name ) ]
    b"trailer" => TRAILER, Trailer;
    b"transfer-encoding" => TRANSFER_ENCODING, TransferEncoding;
    // User-Agent = product *( RWS ( product / comment ) )
    b"user-agent" => USER_AGENT, UserAgent;
    // Upgrade = [ protocol *( OWS "," OWS protocol ) ]
    b"upgrade" => UPGRADE, Upgrade;
    b"upgrade-insecure-requests" => UPGRADE_INSECURE_REQUESTS,
        UpgradeInsecureRequests;
    // Vary = [ ( "*" / field-name ) *( OWS "," OWS ( "*" / field-name ) ) ]
    b"vary" => VARY, Vary;
    // Via = [ ( received-protocol RWS received-by [ RWS comment ] ) *( OWS
    // "," OWS ( received-protocol RWS received-by [ RWS comment ] ) ) ]
    b"via" => VIA, Via;
    b"want-digest" => WANT_DIGEST, WantDigest;
    b"warning" => WARNING, Warning;
    // WWW-Authenticate = [ challenge *( OWS "," OWS challenge ) ]
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

use std::{borrow::Cow, fmt};

impl_header_names! {
    Accept:
        "Accept", b"accept";
    AcceptCharset:
        "Accept-Charset", b"accept-charset";
    AcceptDatetime:
        "Accept-Datetime", b"accept-datetime";
    AcceptEncoding:
        "Accept-Encoding", b"accept-encoding";
    AcceptLanguage:
        "Accept-Language", b"accept-language";
    AcceptPatch:
        "Accept-Patch", b"accept-patch";
    AcceptPost:
        "Accept-Post", b"accept-post";
    AcceptRanges:
        "Accept-Ranges", b"accept-ranges";
    AccessControlAllowCredentials:
        "Access-Control-Allow-Credentials", b"access-control-allow-credentials";
    AccessControlAllowHeaders:
        "Access-Control-Allow-Headers", b"access-control-allow-headers";
    AccessControlAllowMethods:
        "Access-Control-Allow-Methods", b"access-control-allow-methods";
    AccessControlAllowOrigin:
        "Access-Control-Allow-Origin", b"access-control-allow-origin";
    AccessControlExposeHeaders:
        "Access-Control-Expose-Headers", b"access-control-expose-headers";
    AccessControlMaxAge:
        "Access-Control-Max-Age", b"access-control-max-age";
    AccessControlRequestHeaders:
        "Access-Control-Request-Headers", b"access-control-request-headers";
    AccessControlRequestMethod:
        "Access-Control-Request-Method", b"access-control-request-method";
    Age:
        "Age", b"age";
    Allow:
        "Allow", b"allow";
    AltSvc:
        "Alt-Svc", b"alt-svc";
    Authorization:
        "Authorization", b"authorization";
    CacheControl:
        "Cache-Control", b"cache-control";
    CacheStatus:
        "Cache-Status", b"cache-status";
    CdnCacheControl:
        "Cdn-Cache-Control", b"cdn-cache-control";
    ClearSiteData:
        "Clear-Site-Data", b"clear-site-data";
    Connection:
        "Connection", b"connection";
    ContentDisposition:
        "Content-Disposition", b"content-disposition";
    ContentEncoding:
        "Content-Encoding", b"content-encoding";
    ContentLanguage:
        "Content-Language", b"content-language";
    ContentLength:
        "Content-Length", b"content-length";
    ContentLocation:
        "Content-Location", b"content-location";
    ContentRange:
        "Content-Range", b"content-range";
    ContentSecurityPolicy:
        "Content-Security-Policy", b"content-security-policy";
    ContentSecurityPolicyReportOnly:
        "Content-Security-Policy-Report-Only", b"content-security-policy-report-only";
    ContentType:
        "Content-Type", b"content-type";
    Cookie:
        "Cookie", b"cookie";
    CrossOriginEmbedderPolicy:
        "Cross-Origin-Embedder-Policy", b"cross-origin-embedder-policy";
    CrossOriginOpenerPolicy:
        "Cross-Origin-Opener-Policy", b"cross-origin-opener-policy";
    CrossOriginResourcePolicy:
        "Cross-Origin-Resource-Policy", b"cross-origin-resource-policy";
    Date:
        "Date", b"date";
    DeltaBase:
        "Delta-Base", b"delta-base";
    DeviceMemory:
        "Device-Memory", b"device-memory";
    Digest:
        "Digest", b"digest";
    Dnt:
        "Dnt", b"dnt";
    Etag:
        "Etag", b"etag";
    Expect:
        "Expect", b"expect";
    Expires:
        "Expires", b"expires";
    Forwarded:
        "Forwarded", b"forwarded";
    From:
        "From", b"from";
    Host:
        "Host", b"host";
    IfMatch:
        "If-Match", b"http2-settings";
    IfModifiedSince:
        "If-Modified-Since", b"if-match";
    IfNoneMatch:
        "If-None-Match", b"if-modified-since";
    IfRange:
        "If-Range", b"if-none-match";
    IfUnmodifiedSince:
        "If-Unmodified-Since", b"if-range";
    Http2Settings:
        "Http2-Settings", b"if-unmodified-since";
    KeepAlive:
        "Keep-Alive", b"keep-alive";
    LastModified:
        "Last-Modified", b"last-modified";
    Link:
        "Link", b"link";
    Location:
        "Location", b"location";
    MaxForwards:
        "Max-Forwards", b"max-forwards";
    Origin:
        "Origin", b"origin";
    PermissionsPolicy:
        "Permissions-Policy", b"permissions-policy";
    Pragma:
        "Pragma", b"pragma";
    Prefer:
        "Prefer", b"prefer";
    ProxyAuthenticate:
        "Proxy-Authenticate", b"proxy-authenticate";
    ProxyAuthorization:
        "Proxy-Authorization", b"proxy-authorization";
    PublicKeyPins:
        "Public-Key-Pins", b"public-key-pins";
    PublicKeyPinsReportOnly:
        "Public-Key-Pins-Report-Only", b"public-key-pins-report-only";
    Purpose:
        "Purpose", b"purpose";
    Range:
        "Range", b"range";
    Referer:
        "Referer", b"referer";
    ReferrerPolicy:
        "Referrer-Policy", b"referrer-policy";
    Refresh:
        "Refresh", b"refresh";
    RetryAfter:
        "Retry-After", b"retry-after";
    SecChUa:
        "Sec-Ch-Ua", b"sec-ch-ua";
    SecChUaMobile:
        "Sec-Ch-Ua-Mobile", b"sec-ch-ua-mobile";
    SecChUaPlatform:
        "Sec-Ch-Ua-Platform", b"sec-ch-ua-platform";
    SaveData:
        "Save-Data", b"save-data";
    SecFetchDest:
        "Sec-Fetch-Dest", b"sec-fetch-dest";
    SecFetchMode:
        "Sec-Fetch-Mode", b"sec-fetch-mode";
    SecFetchSite:
        "Sec-Fetch-Site", b"sec-fetch-site";
    SecFetchUser:
        "Sec-Fetch-User", b"sec-fetch-user";
    SecGpc:
        "Sec-Gpc", b"sec-gpc";
    SecWebSocketAccept:
        "Sec-Websocket-Accept", b"sec-websocket-accept";
    SecWebSocketExtensions:
        "Sec-Websocket-Extensions", b"sec-websocket-extensions";
    SecWebSocketKey:
        "Sec-Websocket-Key", b"sec-websocket-key";
    SecWebSocketProtocol:
        "Sec-Websocket-Protocol", b"sec-websocket-protocol";
    SecWebSocketVersion:
        "Sec-Websocket-Version", b"sec-websocket-version";
    Server:
        "Server", b"server";
    ServerTiming:
        "Server-Timing", b"server-timing";
    SetCookie:
        "Set-Cookie", b"set-cookie";
    Sourcemap:
        "Sourcemap", b"sourcemap";
    StrictTransportSecurity:
        "Strict-Transport-Security", b"strict-transport-security";
    Te:
        "Te", b"te";
    TimingAllowOrigin:
        "Timing-Allow-Origin", b"timing-allow-origin";
    Trailer:
        "Trailer", b"trailer";
    TransferEncoding:
        "Transfer-Encoding", b"transfer-encoding";
    UserAgent:
        "User-Agent", b"user-agent";
    Upgrade:
        "Upgrade", b"upgrade";
    UpgradeInsecureRequests:
        "Upgrade-Insecure-Requests", b"upgrade-insecure-requests";
    Vary:
        "Vary", b"vary";
    Via:
        "Via", b"via";
    WantDigest:
        "Want-Digest", b"want-digest";
    Warning:
        "Warning", b"warning";
    WwwAuthenticate:
        "Www-Authenticate", b"www-authenticate";
    XContentTypeOptions:
        "X-Content-Type-Options", b"x-content-type-options";
    XDnsPrefetchControl:
        "X-Dns-Prefetch-Control", b"x-dns-prefetch-control";
    XForwardedFor:
        "X-Forwarded-For", b"x-forwarded-for";
    XForwardedHost:
        "X-Forwarded-Host", b"x-forwarded-host";
    XForwardedProto:
        "X-Forwarded-Proto", b"x-forwarded-proto";
    XFrameOptions:
        "X-Frame-Options", b"x-frame-options";
    XPoweredBy:
        "X-Powered-By", b"x-powered-by";
    XRequestId:
        "X-Request-Id", b"x-request-id";
    XRobotsTag:
        "X-Robots-Tag", b"x-robots-tag";
    XUaCompatible:
        "X-Ua-Compatible", b"x-ua-compatible";
    XXssProtection:
        "X-Xss-Protection", b"x-xss-protection";
}

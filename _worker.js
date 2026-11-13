//#region src/languagePage.ts
/** The address of each language page. English is the root page, which other visitors also get. */
var LANGUAGE_PATHS = {
	en: "/",
	"zh-Hans": "/zh-hans/",
	"zh-Hant": "/zh-hant/",
	ja: "/ja/",
	ko: "/ko/"
};
/** The cookie that keeps the language that the visitor chose. The edge worker reads it. */
var LANGUAGE_COOKIE = "grkit_language";
var CODES = Object.keys(LANGUAGE_PATHS);
function isLanguage(value) {
	return CODES.some((code) => code === value);
}
/** Countries where Traditional Chinese is the usual script. */
var TRADITIONAL = /* @__PURE__ */ new Set([
	"TW",
	"HK",
	"MO"
]);
/**
* Maps language tags, most preferred first, to a supported language. Traditional script wins
* for TW, HK, and MO. A bare `zh` takes the script of `country` when the caller knows it.
*/
function matchLanguage(tags, country) {
	for (const tag of tags) {
		const lower = tag.toLowerCase();
		if (lower === "zh" || lower.startsWith("zh-")) {
			if (/hant|-(tw|hk|mo)\b/.test(lower)) return "zh-Hant";
			if (/hans|-(cn|sg|my)\b/.test(lower)) return "zh-Hans";
			return country && TRADITIONAL.has(country) ? "zh-Hant" : "zh-Hans";
		}
		if (lower === "ja" || lower.startsWith("ja-")) return "ja";
		if (lower === "ko" || lower.startsWith("ko-")) return "ko";
		if (lower === "en" || lower.startsWith("en-")) return "en";
	}
	return "en";
}
/** The tags of an Accept-Language header, most preferred first, without `*` and q=0 entries. */
function acceptedLanguages(header) {
	return header.split(",").map((part, index) => {
		const [tag = "", ...params] = part.split(";").map((item) => item.trim());
		const q = params.find((param) => param.startsWith("q="));
		const quality = q === void 0 ? 1 : Number(q.slice(2));
		return {
			tag,
			quality: Number.isFinite(quality) ? quality : 0,
			index
		};
	}).filter((item) => item.tag !== "" && item.tag !== "*" && item.quality > 0).sort((a, b) => b.quality - a.quality || a.index - b.index).map((item) => item.tag);
}
/** The value of the cookie `name` in a Cookie header or `document.cookie`. */
function cookieValue(cookies, name) {
	for (const part of cookies.split(";")) {
		const [key, ...value] = part.trim().split("=");
		if (key === name) return value.join("=");
	}
	return null;
}
/** The language of the countries where most visitors read it. */
var COUNTRY_LANGUAGES = {
	CN: "zh-Hans",
	TW: "zh-Hant",
	HK: "zh-Hant",
	MO: "zh-Hant",
	JP: "ja",
	KR: "ko"
};
/**
* The language page for a visitor of the root page. A language that the visitor chose comes
* first, then the browser languages. The country of the IP address decides only when the
* browser sends no language, and it picks the script of a bare `zh`.
*/
function visitorLanguage({ cookie, acceptLanguage, country }) {
	const chosen = cookie === null ? null : cookieValue(cookie, LANGUAGE_COOKIE);
	if (isLanguage(chosen)) return chosen;
	const tags = acceptedLanguages(acceptLanguage ?? "");
	if (tags.length > 0) return matchLanguage(tags, country);
	return country && COUNTRY_LANGUAGES[country] || "en";
}
//#endregion
//#region src/edge.ts
var edge_default = { async fetch(request, env) {
	const url = new URL(request.url);
	if (url.pathname === "/" && (request.method === "GET" || request.method === "HEAD")) {
		const language = visitorLanguage({
			cookie: request.headers.get("Cookie"),
			acceptLanguage: request.headers.get("Accept-Language"),
			country: request.cf?.country ?? null
		});
		if (language !== "en") {
			url.pathname = LANGUAGE_PATHS[language];
			return new Response(null, {
				status: 302,
				headers: {
					Location: url.href,
					"Cache-Control": "private, no-store",
					Vary: "Accept-Language, Cookie"
				}
			});
		}
	}
	const asset = await env.ASSETS.fetch(request);
	const response = new Response(asset.body, asset);
	for (const [name, value] of Object.entries({
		"Content-Security-Policy": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval' https://plausible.techflow.studio 'sha256-/6SBPqW+GW+//4nlXX6Y1nR9dWlh0gsQJ6KK71djH6A='; style-src 'self' 'unsafe-hashes' 'sha256-NamvEQbYOZLzIpvbEHTXsTsIEFfHGOX+IkN82LVT1A4=' 'sha256-ZnrWpQYDFI/TmwMC1PkPM0pRo2op5vmZPUnbCzBEXD0=' 'sha256-tPlu9AUDHlHy+I+j2qOHmW810JQa5vHdQap4rO0I59k=' 'sha256-cepK6p8WVdBtHCueDUXuGxUlG6robCb/RhMcwF1lKJU=' 'sha256-2iC95E8wv/B7hf3E+gbxGBwTBo83Rwmt8s/y1Pulcnw=' 'sha256-VL7z6fdRNANnD+G6b64pPfmMjjGi2NCfDrG4zBPiy7Y=' 'sha256-lJxWhGoILzz/6e3wBSUELVQP325OD8VPBzJ8Bhfw9M0=' 'sha256-TqiaSclY5fZtAPCH8eES4vFc20E9a6VZ3SkKB3xlYk0=' 'sha256-ZPHQZ0i1zFXIZnZ4coyednH35d3rskV/n+C5TZ7wKzY=' 'sha256-vo8361nuOl48rs065DnLkKoPz37YSlP/GcusHtuGGPc=' 'sha256-yr0/IQmNLUSp+AWFXCgMHOxbnQ4Z72ThGNasJm81g3k=' 'sha256-Qs5gmTPtJViW9Bmq4RB4AU/jKCGGmFkna2A/kkC0/JE=' 'sha256-e+asQ9hujND2SutjeuMFYaiQA8/9siO6/BfQb0Qu/wo=' 'sha256-vpCwT/2DchmpARhQZRaiOxKh0gKarLiUXAqnPk9QWt0='; font-src 'self'; img-src 'self' blob: data:; connect-src 'self' https://plausible.techflow.studio; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
		"X-Content-Type-Options": "nosniff",
		"Referrer-Policy": "strict-origin-when-cross-origin",
		"Permissions-Policy": "camera=(), microphone=(), geolocation=()"
	})) if (!response.headers.has(name)) response.headers.set(name, value);
	response.headers.append("Vary", "Accept-Language, Cookie");
	return response;
} };
//#endregion
export { edge_default as default };

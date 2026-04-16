import { action } from "@engine";
import type { ActionContext } from "@engine/types";
import config from "./track-config";

type TrackConfig = {
	endpoint: string;
	service?: string;
	release?: string;
};

type TrackContext = {
	view_id: string;
	pathname: string;
	search?: string;
	hash?: string;
};

type PageViewItem = {
	kind: "page_view";
	timestamp_ms: number;
	context: TrackContext;
	referrer?: string;
	navigation_type?: NavigationTimingType;
};

type AnalyticsItem = {
	kind: "analytics";
	timestamp_ms: number;
	props: unknown;
	context: TrackContext;
};

type ExceptionItem = {
	kind: "exception";
	timestamp_ms: number;
	message: string;
	stack?: string;
	context: TrackContext;
};

type TrackItem = PageViewItem | AnalyticsItem | ExceptionItem;

type NormalizedError = {
	message: string;
	stack?: string;
};

let flushTimer: number | undefined;
let pageViewSent = false;

function randomHex(bytes: number): string {
	const values = new Uint8Array(bytes);
	crypto.getRandomValues(values);
	return Array.from(values, (value) =>
		value.toString(16).padStart(2, "0"),
	).join("");
}

const VIEW_ID = randomHex(16);

function context(): TrackContext {
	return {
		view_id: VIEW_ID,
		pathname: window.location.pathname,
		...(window.location.search ? { search: window.location.search } : {}),
		...(window.location.hash ? { hash: window.location.hash } : {}),
	};
}

const TRACK_QUEUE: TrackItem[] = [];
const CONFIG: TrackConfig = config;

function enqueue(item: TrackItem): void {
	TRACK_QUEUE.push(item);
	if (flushTimer !== undefined) {
		return;
	}

	flushTimer = window.setTimeout(() => {
		flushTimer = undefined;
		flush(false);
	}, 500);
}

let flushInFlight: Promise<void> | null = null;

function flush(useKeepalive: boolean): void {
	if (flushInFlight) return;
	if (TRACK_QUEUE.length === 0) return;

	const items = TRACK_QUEUE.splice(0, TRACK_QUEUE.length);
	const body = JSON.stringify({
		service: CONFIG.service,
		release: CONFIG.release,
		sent_at_ms: Date.now(),
		items,
	});

	let succeeded = false;

	flushInFlight = fetch(CONFIG.endpoint, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body,
		keepalive: useKeepalive && body.length <= 60_000,
	})
		.then((response) => {
			if (!response.ok) {
				TRACK_QUEUE.unshift(...items);
			} else {
				succeeded = true;
			}
		})
		.catch(() => {
			TRACK_QUEUE.unshift(...items);
		})
		.finally(() => {
			flushInFlight = null;
			if (TRACK_QUEUE.length === 0) {
				return;
			}

			if (succeeded) {
				flush(false);
			} else if (flushTimer === undefined) {
				flushTimer = window.setTimeout(() => {
					flushTimer = undefined;
					flush(false);
				}, 500);
			}
		});
}

function getNav(): PerformanceNavigationTiming | null {
	const entry = performance.getEntriesByType("navigation")[0];
	return entry instanceof PerformanceNavigationTiming ? entry : null;
}

function trackPageView(): void {
	if (pageViewSent) {
		return;
	}
	pageViewSent = true;

	const nav = getNav();

	enqueue({
		kind: "page_view",
		timestamp_ms: Date.now(),
		context: context(),
		referrer: document.referrer,
		...(nav?.type ? { navigation_type: nav.type } : {}),
	});
}

function normalizeError(reason: unknown): NormalizedError {
	if (reason === undefined) {
		return {
			message: "undefined",
		};
	}
	if (reason === null) {
		return {
			message: "null",
		};
	}

	if (reason instanceof Error) {
		return {
			message: reason.message || String(reason),
			...(reason.stack ? { stack: reason.stack } : {}),
		};
	}

	if (typeof reason === "string") {
		return {
			message: reason,
		};
	}

	try {
		const message = JSON.stringify(reason);
		if (typeof message === "string") {
			return {
				message,
			};
		}

		return {
			message: String(reason),
		};
	} catch {
		return {
			message: String(reason),
		};
	}
}

function trackException(message: string, stack?: string): void {
	enqueue({
		kind: "exception",
		timestamp_ms: Date.now(),
		message,
		stack,
		context: context(),
	});
}

action({
	name: "track",
	apply(_: ActionContext, props: unknown = {}): void {
		enqueue({
			kind: "analytics",
			timestamp_ms: Date.now(),
			props,
			context: context(),
		});
	},
});

window.addEventListener("error", (event) => {
	const error = normalizeError(event.error ?? event.message ?? "Unknown error");
	trackException(error.message, error.stack);
});

window.addEventListener("unhandledrejection", (event) => {
	const error = normalizeError(event.reason);
	trackException(error.message, error.stack);
});

window.addEventListener("pagehide", () => flush(true));
document.addEventListener("visibilitychange", () => {
	if (document.visibilityState === "hidden") {
		flush(true);
	}
});

if (document.readyState === "loading") {
	document.addEventListener("DOMContentLoaded", trackPageView, {
		once: true,
	});
} else {
	trackPageView();
}

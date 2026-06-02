// ==UserScript==
// @name         Work Review Via Logger
// @namespace    https://work-review.local/
// @version      0.1.0
// @description  Record Via page lifecycle events for Work Review Mobile.
// @match        http://*/*
// @match        https://*/*
// @run-at       document-start
// ==/UserScript==

(function () {
  "use strict";

  const endpoint = "http://127.0.0.1:17890/log";
  const cacheKey = "work_review_via_logger_queue_v1";
  const dedupeWindowMs = 800;
  let currentUrl = location.href;
  let enterTs = Date.now();
  let lastSentKey = "";
  let lastSentTs = 0;

  function payload(eventType, durationMs) {
    return {
      ts: Date.now(),
      event_type: eventType,
      url: location.href,
      title: document.title || "",
      referrer: document.referrer || "",
      visible: document.visibilityState === "visible",
      duration_ms: Math.max(0, durationMs || 0),
      user_agent: navigator.userAgent,
      source: "via_userscript"
    };
  }

  function isDuplicate(item) {
    const key = [item.event_type, item.url, item.title].join("|");
    const now = Date.now();
    if (key === lastSentKey && now - lastSentTs < dedupeWindowMs) return true;
    lastSentKey = key;
    lastSentTs = now;
    return false;
  }

  function readQueue() {
    try {
      return JSON.parse(localStorage.getItem(cacheKey) || "[]");
    } catch (_) {
      return [];
    }
  }

  function writeQueue(queue) {
    try {
      localStorage.setItem(cacheKey, JSON.stringify(queue.slice(-100)));
    } catch (_) {
      // Ignore storage failures; page logging must not break browsing.
    }
  }

  function enqueue(item) {
    const queue = readQueue();
    queue.push(item);
    writeQueue(queue);
  }

  function post(item) {
    if (isDuplicate(item)) return;
    const body = JSON.stringify(item);
    let sent = false;
    try {
      if (navigator.sendBeacon) {
        sent = navigator.sendBeacon(endpoint, new Blob([body], { type: "application/json" }));
      }
    } catch (_) {
      sent = false;
    }
    if (sent) return;

    fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
      keepalive: true
    }).catch(function () {
      enqueue(item);
    });
  }

  function flushQueue() {
    const queue = readQueue();
    if (!queue.length) return;
    writeQueue([]);
    queue.forEach(post);
  }

  function leaveCurrent(reason) {
    post(payload(reason || "page_leave", Date.now() - enterTs));
  }

  function enterCurrent(reason) {
    currentUrl = location.href;
    enterTs = Date.now();
    post(payload(reason || "page_enter", 0));
  }

  function handleUrlChange() {
    if (location.href === currentUrl) return;
    leaveCurrent("page_leave");
    enterCurrent("page_enter");
  }

  const originalPushState = history.pushState;
  history.pushState = function () {
    const result = originalPushState.apply(this, arguments);
    setTimeout(handleUrlChange, 0);
    return result;
  };

  const originalReplaceState = history.replaceState;
  history.replaceState = function () {
    const result = originalReplaceState.apply(this, arguments);
    setTimeout(handleUrlChange, 0);
    return result;
  };

  window.addEventListener("popstate", function () {
    setTimeout(handleUrlChange, 0);
  });

  document.addEventListener("visibilitychange", function () {
    if (document.visibilityState === "hidden") {
      leaveCurrent("page_hidden");
    } else {
      enterTs = Date.now();
      post(payload("page_visible", 0));
    }
  });

  window.addEventListener("beforeunload", function () {
    leaveCurrent("page_unload");
  });

  flushQueue();
  enterCurrent("page_enter");
})();

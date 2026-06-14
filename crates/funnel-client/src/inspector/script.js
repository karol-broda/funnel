(() => {
  const list = document.getElementById("request-list");
  if (!list) return;

  const root = document.documentElement;
  const themeToggle = document.querySelector(".theme-toggle");
  if (themeToggle) {
    themeToggle.addEventListener("click", () => {
      const next = root.dataset.theme === "dark" ? "light" : "dark";
      root.dataset.theme = next;
      localStorage.setItem("funnel-inspector-theme", next);
    });
  }

  const filter = list.dataset.filter || "all";
  let query = list.dataset.query || "";
  let navigationSequence = 0;

  function currentRequestId() {
    return new URL(window.location.href).searchParams.get("request");
  }

  function requestHref(id) {
    const url = new URL("/", window.location.origin);
    url.searchParams.set("request", id);
    if (filter && filter !== "all") url.searchParams.set("filter", filter);
    if (query) url.searchParams.set("q", query);
    return `${url.pathname}${url.search}`;
  }

  function syncUrl() {
    const url = new URL(window.location.href);
    if (query) url.searchParams.set("q", query);
    else url.searchParams.delete("q");
    if (filter && filter !== "all") url.searchParams.set("filter", filter);
    else url.searchParams.delete("filter");
    history.replaceState(null, "", `${url.pathname}${url.search}`);
  }

  function syncRowHrefs() {
    for (const row of list.children) {
      if (row.dataset.id) row.href = requestHref(row.dataset.id);
    }
  }

  function syncActiveRow(selectedId) {
    const selected =
      selectedId || currentRequestId() || list.querySelector(".request-row.active")?.dataset.id;
    for (const row of list.children) {
      row.classList.toggle("active", Boolean(selected && row.dataset.id === selected));
    }
  }

  function applyQuery() {
    const normalized = query.toLowerCase();
    for (const row of list.children) {
      const haystack = row.textContent.toLowerCase();
      row.style.display = !normalized || haystack.includes(normalized) ? "" : "none";
    }
  }
  applyQuery();
  syncActiveRow();

  const searchInput = document.getElementById("search");
  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      query = e.target.value;
      applyQuery();
      syncUrl();
      syncRowHrefs();
    });
  }

  function syncHeaderEditor(editor) {
    const lines = [];
    for (const row of editor.querySelectorAll(".headers-editor-row")) {
      const name = row.querySelector(".header-name")?.value.trim();
      const value = row.querySelector(".header-value")?.value.trim();
      if (name) lines.push(`${name}: ${value || ""}`);
    }
    const raw = editor.querySelector(".headers-editor");
    if (raw) raw.value = lines.join("\n");
  }

  function bindHeaderEditor(editor) {
    if (editor.dataset.bound === "true") return;
    editor.dataset.bound = "true";

    const rows = editor.querySelector(".headers-editor-rows");
    const add = editor.querySelector(".add-header");
    const template = () =>
      `<div class="headers-editor-row">` +
      `<input class="input header-name" placeholder="content-type">` +
      `<input class="input header-value" placeholder="application/json">` +
      `<button class="btn subtle remove-header" type="button" aria-label="Remove header">Remove</button>` +
      `</div>`;

    editor.addEventListener("input", () => syncHeaderEditor(editor));
    editor.addEventListener("click", (event) => {
      const remove = event.target.closest(".remove-header");
      if (remove) {
        remove.closest(".headers-editor-row")?.remove();
        syncHeaderEditor(editor);
      }
    });
    add?.addEventListener("click", () => {
      rows?.insertAdjacentHTML("beforeend", template());
      syncHeaderEditor(editor);
    });
  }

  function bindDetail(rootNode = document) {
    for (const editor of rootNode.querySelectorAll("[data-header-editor]")) {
      bindHeaderEditor(editor);
      syncHeaderEditor(editor);
    }
  }

  bindDetail();

  function formPayload(form) {
    for (const editor of form.querySelectorAll("[data-header-editor]")) {
      syncHeaderEditor(editor);
    }
    const data = new FormData(form);
    return {
      method: String(data.get("method") || "GET"),
      path: String(data.get("path") || "/"),
      headers: String(data.get("headers") || ""),
      body: String(data.get("body") || ""),
    };
  }

  async function submitReplay(form) {
    const status = form.querySelector(".replay-status");
    const button = form.querySelector('button[type="submit"]');
    const originalLabel = button?.textContent || "";

    if (status) status.textContent = "Sending...";
    if (button) {
      button.disabled = true;
      button.textContent = "Sending";
    }

    try {
      const response = await fetch(form.dataset.apiAction, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(formPayload(form)),
      });

      if (!response.ok) {
        const message = await response.text();
        throw new Error(message || `Replay failed (${response.status})`);
      }

      const exchange = await response.json();
      if (status) status.textContent = "Sent";
      navigateToRequest(requestHref(exchange.id), true);
    } catch (error) {
      if (status) status.textContent = error.message || "Replay failed";
      if (button) {
        button.disabled = false;
        button.textContent = originalLabel;
      }
    }
  }

  document.addEventListener("submit", (event) => {
    const form = event.target.closest(".replay-form[data-api-action]");
    if (!form) return;

    event.preventDefault();
    submitReplay(form);
  });

  document.addEventListener("click", (event) => {
    const button = event.target.closest(".format-json");
    if (!button) return;

    const form = button.closest("form");
    const editor = form?.querySelector(".body-editor");
    const status = form?.querySelector(".replay-status");
    if (!editor) return;

    try {
      editor.value = JSON.stringify(JSON.parse(editor.value), null, 2);
      if (status) status.textContent = "Formatted JSON";
    } catch {
      if (status) status.textContent = "Body is not valid JSON";
    }
  });

  async function navigateToRequest(href, push) {
    const sequence = ++navigationSequence;
    const url = new URL(href, window.location.origin);
    const detail = document.querySelector(".detail-column");
    if (!detail) {
      window.location.href = url.href;
      return;
    }

    detail.setAttribute("aria-busy", "true");

    try {
      const response = await fetch(url.href, {
        headers: { accept: "text/html" },
      });
      if (!response.ok) throw new Error(`Request failed (${response.status})`);

      const html = await response.text();
      const doc = new DOMParser().parseFromString(html, "text/html");
      const nextDetail = doc.querySelector(".detail-column");
      if (!nextDetail) throw new Error("Detail column not found");
      if (sequence !== navigationSequence) return;

      detail.innerHTML = nextDetail.innerHTML;
      detail.scrollTop = 0;
      bindDetail(detail);

      if (push) {
        history.pushState(null, "", `${url.pathname}${url.search}`);
      } else {
        history.replaceState(null, "", `${url.pathname}${url.search}`);
      }
      syncRowHrefs();
      syncActiveRow(
        url.searchParams.get("request") || doc.querySelector(".request-row.active")?.dataset.id,
      );
    } catch (error) {
      if (sequence !== navigationSequence) return;
      console.warn("inspector: falling back to full navigation", error);
      window.location.href = url.href;
    } finally {
      if (sequence === navigationSequence) detail.removeAttribute("aria-busy");
    }
  }

  function statusClass(status) {
    if (status >= 500) return "s5";
    if (status >= 400) return "s4";
    return "s2";
  }

  function escapeHtml(value) {
    return String(value ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");
  }

  function escapeAttr(value) {
    return escapeHtml(value).replaceAll("`", "&#96;");
  }

  function matches(exchange) {
    const status = Number(exchange.status);
    let ok = true;
    if (filter === "2xx") ok = status >= 200 && status < 300;
    else if (filter === "4xx") ok = status >= 400 && status < 500;
    else if (filter === "5xx") ok = status >= 500;
    else if (filter === "replay") ok = exchange.source === "replay";
    if (!ok) return false;
    const normalized = query.toLowerCase();
    if (!normalized) return true;
    const haystack = `${exchange.method} ${exchange.path} ${exchange.status} ${exchange.source}`.toLowerCase();
    return haystack.includes(normalized);
  }

  function rowHtml(exchange) {
    const cls = statusClass(Number(exchange.status));
    const path = exchange.path || "/";
    const href = requestHref(exchange.id);
    const display = matches(exchange) ? "" : "none";
    const active = currentRequestId() === exchange.id ? " active" : "";
    return `<a class="request-row${active}" href="${href}" data-id="${escapeAttr(exchange.id)}" style="display: ${display}">` +
      `<span class="method-pill">${escapeHtml(exchange.method)}</span>` +
      `<span class="request-main">` +
      `<span class="path" title="${escapeAttr(path)}">${escapeHtml(path)}</span>` +
      `<span class="sub">${escapeHtml(exchange.source)} - ${escapeHtml(exchange.duration_ms)}ms</span>` +
      `</span>` +
      `<span class="status ${cls}">${escapeHtml(exchange.status)}</span>` +
      `</a>`;
  }

  function prepend(exchange) {
    let existing = null;
    for (const row of list.children) {
      if (row.dataset.id === exchange.id) {
        existing = row;
        break;
      }
    }
    if (existing) existing.remove();
    list.insertAdjacentHTML("afterbegin", rowHtml(exchange));
    while (list.children.length > 250) {
      list.lastElementChild?.remove();
    }
    syncRowHrefs();
    syncActiveRow();
  }

  function onEvent(event) {
    try {
      const payload = JSON.parse(event.data);
      if (payload && payload.exchange) prepend(payload.exchange);
    } catch (error) {
      console.warn("inspector: failed to parse exchange event", error);
    }
  }

  const source = new EventSource("/api/events");
  source.addEventListener("exchange", onEvent);
  source.onmessage = onEvent;

  list.addEventListener("click", (event) => {
    const link = event.target.closest(".request-row");
    if (
      !link ||
      event.defaultPrevented ||
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey
    ) {
      return;
    }

    event.preventDefault();
    navigateToRequest(link.href, true);
  });

  window.addEventListener("popstate", () => {
    navigateToRequest(window.location.href, false);
  });

  for (const link of document.querySelectorAll(".filters .filter")) {
    link.addEventListener("click", (e) => {
      e.preventDefault();
      const url = new URL(link.href);
      if (searchInput && searchInput.value) {
        url.searchParams.set("q", searchInput.value);
      } else {
        url.searchParams.delete("q");
      }
      window.location.href = url.href;
    });
  }
})();

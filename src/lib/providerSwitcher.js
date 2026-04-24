// @ts-nocheck
(function () {
  if (typeof globalThis === "undefined") return;
  if (globalThis.__promptheusSwitcher) return;

  var EXTERNAL_LINK_SVG =
    '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>';
  var CHECK_SVG =
    '<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
  var CARET_SVG =
    '<svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>';

  var FONT_STACK =
    "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

  function setStyle(el, styles) {
    for (var k in styles) el.style.setProperty(k, styles[k]);
  }

  function createProviderSwitcher(opts) {
    var providers = (opts && opts.providers) || [];
    var activeId = opts && opts.activeId;
    var onSelect = (opts && opts.onSelect) || function () {};
    var onOpenNewWindow =
      (opts && opts.onOpenNewWindow) || function () {};
    var newWindowTitle =
      (opts && opts.newWindowTitle) || "Open in new window";

    var wrap = document.createElement("div");
    wrap.className = "__promptheus-switcher";
    setStyle(wrap, {
      position: "relative",
      display: "inline-block",
      "font-family": FONT_STACK,
      "font-size": "12px",
      "line-height": "1.2",
    });

    var trigger = document.createElement("button");
    trigger.type = "button";
    setStyle(trigger, {
      display: "inline-flex",
      "align-items": "center",
      gap: "4px",
      height: "28px",
      padding: "0 10px",
      background: "rgba(255,255,255,0.06)",
      color: "#e0e0e0",
      border: "1px solid rgba(255,255,255,0.12)",
      "border-radius": "6px",
      font: "500 12px/1.2 " + FONT_STACK,
      cursor: "pointer",
      "user-select": "none",
      "white-space": "nowrap",
    });
    trigger.addEventListener("mouseover", function () {
      trigger.style.setProperty("background", "rgba(255,255,255,0.12)");
    });
    trigger.addEventListener("mouseout", function () {
      trigger.style.setProperty("background", "rgba(255,255,255,0.06)");
    });
    trigger.addEventListener("click", function (e) {
      e.stopPropagation();
      toggle();
    });
    updateTrigger();
    wrap.appendChild(trigger);

    var menu = document.createElement("div");
    setStyle(menu, {
      position: "absolute",
      top: "calc(100% + 4px)",
      left: "0",
      "min-width": "180px",
      background: "#2a2a2a",
      border: "1px solid rgba(255,255,255,0.15)",
      "border-radius": "6px",
      "box-shadow": "0 4px 12px rgba(0,0,0,0.3)",
      padding: "4px 0",
      display: "none",
      "z-index": "2147483647",
    });

    var rows = [];
    providers.forEach(function (p) {
      rows.push(createRow(p, menu));
    });
    wrap.appendChild(menu);

    function updateTrigger() {
      var current = providers.find(function (p) {
        return p.id === activeId;
      });
      trigger.innerHTML = "";
      var nameSpan = document.createElement("span");
      nameSpan.textContent = current ? current.name : "—";
      trigger.appendChild(nameSpan);
      var caret = document.createElement("span");
      caret.innerHTML = CARET_SVG;
      setStyle(caret, {
        display: "inline-flex",
        "align-items": "center",
        opacity: "0.7",
      });
      trigger.appendChild(caret);
    }

    function createRow(p, parent) {
      var row = document.createElement("div");
      setStyle(row, {
        display: "flex",
        "align-items": "stretch",
      });
      row.addEventListener("mouseover", function () {
        row.style.setProperty("background", "rgba(255,255,255,0.08)");
      });
      row.addEventListener("mouseout", function () {
        row.style.setProperty("background", "transparent");
      });

      var nameBtn = document.createElement("button");
      nameBtn.type = "button";
      setStyle(nameBtn, {
        display: "flex",
        "align-items": "center",
        gap: "8px",
        flex: "1",
        padding: "7px 12px",
        background: "transparent",
        border: "none",
        color: "#e0e0e0",
        font: "500 13px/1.2 " + FONT_STACK,
        cursor: "pointer",
        "text-align": "left",
        "white-space": "nowrap",
      });

      var nameSpan = document.createElement("span");
      nameSpan.textContent = p.name;
      setStyle(nameSpan, { flex: "1", "white-space": "nowrap" });
      nameBtn.appendChild(nameSpan);

      var checkSpan = document.createElement("span");
      checkSpan.innerHTML = CHECK_SVG;
      setStyle(checkSpan, {
        display: "inline-flex",
        "align-items": "center",
        color: "#7dd3fc",
        visibility: p.id === activeId ? "visible" : "hidden",
      });
      nameBtn.appendChild(checkSpan);

      nameBtn.addEventListener("click", function () {
        close();
        if (p.id === activeId) return;
        onSelect(p.id);
      });
      row.appendChild(nameBtn);

      var newWinBtn = document.createElement("button");
      newWinBtn.type = "button";
      newWinBtn.title = newWindowTitle;
      newWinBtn.innerHTML = EXTERNAL_LINK_SVG;
      setStyle(newWinBtn, {
        display: "flex",
        "align-items": "center",
        "justify-content": "center",
        padding: "0 10px",
        background: "transparent",
        border: "none",
        color: "rgba(255,255,255,0.5)",
        cursor: "pointer",
      });
      newWinBtn.addEventListener("mouseover", function () {
        newWinBtn.style.setProperty("color", "rgba(255,255,255,0.95)");
      });
      newWinBtn.addEventListener("mouseout", function () {
        newWinBtn.style.setProperty("color", "rgba(255,255,255,0.5)");
      });
      newWinBtn.addEventListener("click", function (e) {
        e.stopPropagation();
        close();
        onOpenNewWindow(p.id);
      });
      row.appendChild(newWinBtn);

      parent.appendChild(row);

      return { providerId: p.id, checkSpan: checkSpan };
    }

    function openMenu() {
      menu.style.setProperty("display", "block");
    }
    function close() {
      menu.style.setProperty("display", "none");
    }
    function toggle() {
      var isOpen = menu.style.getPropertyValue("display") === "block";
      if (isOpen) close();
      else openMenu();
    }

    function onDocumentClick(e) {
      if (!wrap.contains(e.target)) close();
    }
    document.addEventListener("click", onDocumentClick);

    function setActive(id) {
      activeId = id;
      updateTrigger();
      rows.forEach(function (r) {
        r.checkSpan.style.setProperty(
          "visibility",
          r.providerId === id ? "visible" : "hidden",
        );
      });
    }

    function destroy() {
      document.removeEventListener("click", onDocumentClick);
      if (wrap.parentNode) wrap.parentNode.removeChild(wrap);
    }

    return {
      element: wrap,
      setActive: setActive,
      open: openMenu,
      close: close,
      toggle: toggle,
      destroy: destroy,
    };
  }

  globalThis.__promptheusSwitcher = { create: createProviderSwitcher };
})();

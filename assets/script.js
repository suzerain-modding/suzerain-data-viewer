(function () {
  function updateToggleIcon(btn, t) {
    if (!btn) return;
    btn.setAttribute(
      "aria-label",
      "Switch to " + (t === "dark" ? "light" : "dark") + " mode",
    );
    btn.innerHTML =
      t === "dark"
        ? '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>'
        : '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>';
  }

  function openHashTarget() {
    const hash = window.location.hash;
    if (!hash) return;

    const target = document.querySelector(hash);
    if (!(target instanceof HTMLElement)) return;

    const listItem = target.closest(".list-item");
    if (listItem) {
      const toggleButton = listItem.querySelector(".list-item-header .toggle");
      if (toggleButton instanceof HTMLElement) {
        toggleButton.setAttribute("aria-expanded", "true");
      }
      listItem.classList.remove("collapsed");
    }

    requestAnimationFrame(() => {
      target.scrollIntoView({ behavior: "smooth", block: "start" });
    });
  }

  document.addEventListener("DOMContentLoaded", function () {
    openHashTarget();
    window.addEventListener("hashchange", openHashTarget);

    // ── Collapsible toggles ────────────────────────────────
    // Use event delegation so it works for all .toggle buttons, including
    // those inside .collapsible-section-content (section sub-toggles).
    document.addEventListener("click", function (e) {
      const btn = e.target.closest(".toggle");
      if (!btn) return;

      const expanded = btn.getAttribute("aria-expanded") === "true";
      btn.setAttribute("aria-expanded", String(!expanded));

      // For list-item toggles: the collapsible is the .list-item ancestor
      const listItem = btn.closest(".list-item");
      if (listItem && btn.closest(".list-item-header")) {
        listItem.classList.toggle("collapsed", expanded);
        return;
      }

      // For section sub-toggles: the next sibling with .collapsible-section-content
      const label = btn.closest(
        ".entry-section-label, .collapsible-section-label",
      );
      if (label) {
        const section = label.closest(".entry-section");
        if (section) {
          const content = section.querySelector(".collapsible-section-content");
          if (content) {
            content.classList.toggle("collapsed", expanded);
          }
        }
      }
    });
  });
})();

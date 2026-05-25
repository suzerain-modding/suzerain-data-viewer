(function () {
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
      const btn =
        e.target.closest(".toggle") ??
        e.target.closest(".list-item-header")?.querySelector(".toggle");
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

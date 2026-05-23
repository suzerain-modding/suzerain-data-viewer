document.addEventListener("DOMContentLoaded", () => {
  document.querySelectorAll(".toggle").forEach((btn) => {
    btn.addEventListener("click", () => {
      const item = btn.closest(".collapsible");
      if (!item) return;
      const expanded = btn.getAttribute("aria-expanded") === "true";
      btn.setAttribute("aria-expanded", (!expanded).toString());
      item.classList.toggle("collapsed", expanded);
    });
  });
});

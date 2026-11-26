// origami website, both pages: the sticky header gets a crease once the page
// scrolls, the link of the section in view is marked, and on small screens
// the links sit behind a menu button (style.css, "scripting: enabled").

(() => {
  const top = document.querySelector(".top");
  if (!top) return;

  let ticking = false;
  const onScroll = () => {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(() => {
      top.classList.toggle("scrolled", scrollY > 4);
      ticking = false;
    });
  };
  addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  // ---- menu ---------------------------------------------------------------
  const button = top.querySelector(".menu-button");
  const nav = top.querySelector("nav");
  const setOpen = (open, { focus = false } = {}) => {
    button.setAttribute("aria-expanded", String(open));
    nav.classList.toggle("open", open);
    if (!open && focus) button.focus();
  };
  if (button && nav) {
    button.addEventListener("click", () => setOpen(button.getAttribute("aria-expanded") !== "true"));
    nav.addEventListener("click", (e) => {
      if (e.target.closest("a")) setOpen(false);
    });
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && nav.classList.contains("open")) setOpen(false, { focus: true });
    });
    document.addEventListener("pointerdown", (e) => {
      if (nav.classList.contains("open") && !top.contains(e.target)) setOpen(false);
    });
    // Leaving the menu with Tab closes it.
    top.addEventListener("focusout", (e) => {
      if (nav.classList.contains("open") && e.relatedTarget && !top.contains(e.relatedTarget)) setOpen(false);
    });
    matchMedia("(min-width: 52.01rem)").addEventListener("change", () => setOpen(false));
  }

  // ---- section in view ---------------------------------------------------
  const links = [...top.querySelectorAll("nav a[href^='#']")];
  const linkFor = new Map();
  for (const a of links) {
    for (const id of (a.dataset.sections ?? a.getAttribute("href").slice(1)).split(" ")) linkFor.set(id, a);
  }
  const sections = [...linkFor.keys()].map((id) => document.getElementById(id)).filter(Boolean);
  if (!sections.length || !("IntersectionObserver" in window)) return;
  const visible = new Set();
  const mark = () => {
    // The first section (in page order) that crosses the reading line.
    const current = sections.find((s) => visible.has(s));
    const active = current ? linkFor.get(current.id) : null;
    for (const a of links) {
      if (a === active) a.setAttribute("aria-current", "true");
      else a.removeAttribute("aria-current");
    }
  };
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) visible.add(e.target);
        else visible.delete(e.target);
      }
      mark();
    },
    { rootMargin: "-35% 0px -60% 0px" },
  );
  for (const s of sections) io.observe(s);
})();

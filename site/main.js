// Opens a screenshot in a viewer instead of leaving the page.
const viewer = document.getElementById("viewer");
document.querySelectorAll(".gallery a, .pair a").forEach((link) => {
  link.addEventListener("click", (event) => {
    if (!viewer.showModal) return;
    event.preventDefault();
    const img = link.querySelector("img");
    viewer.querySelector("img").src = link.href;
    viewer.querySelector("img").alt = img.alt;
    viewer.querySelector("p").textContent =
      link.closest("figure").querySelector("figcaption")?.textContent ?? "";
    viewer.showModal();
  });
});
viewer.addEventListener("click", () => viewer.close());

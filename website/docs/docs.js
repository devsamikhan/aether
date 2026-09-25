/**
 * AETHER 2.0 Documentation Portal Interactive Script
 * Search, ScrollSpy, Copy Snippets & Project Domain Filter
 */

document.addEventListener("DOMContentLoaded", () => {
  // 1. Copy Code Block Functionality
  document.querySelectorAll(".copy-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const codeBlock = btn.closest(".code-block");
      const pre = codeBlock.querySelector("pre");
      if (!pre) return;
      
      const textToCopy = pre.innerText;
      navigator.clipboard.writeText(textToCopy).then(() => {
        const originalText = btn.innerHTML;
        btn.innerHTML = `<span>✓ Copied</span>`;
        btn.classList.add("copied");
        setTimeout(() => {
          btn.innerHTML = originalText;
          btn.classList.remove("copied");
        }, 2000);
      });
    });
  });

  // 2. Active ScrollSpy for Left Sidebar & Right TOC
  const sections = document.querySelectorAll("article[id], section[id]");
  const sidebarLinks = document.querySelectorAll(".sidebar-link");
  const tocLinks = document.querySelectorAll(".toc-link");

  function updateActiveLinks() {
    let currentId = "";
    const scrollPos = window.scrollY + 120;

    sections.forEach((sec) => {
      const top = sec.offsetTop;
      const height = sec.offsetHeight;
      if (scrollPos >= top && scrollPos < top + height) {
        currentId = sec.getAttribute("id");
      }
    });

    if (currentId) {
      sidebarLinks.forEach((link) => {
        if (link.getAttribute("href") === `#${currentId}`) {
          link.classList.add("active");
        } else {
          link.classList.remove("active");
        }
      });

      tocLinks.forEach((link) => {
        if (link.getAttribute("href") === `#${currentId}`) {
          link.classList.add("active");
        } else {
          link.classList.remove("active");
        }
      });
    }
  }

  window.addEventListener("scroll", updateActiveLinks, { passive: true });
  updateActiveLinks();

  // 3. Instant Search Filter
  const searchInput = document.getElementById("docsSearchInput");
  if (searchInput) {
    searchInput.addEventListener("input", (e) => {
      const query = e.target.value.toLowerCase().trim();
      const articles = document.querySelectorAll(".doc-section");

      articles.forEach((art) => {
        if (!query) {
          art.style.display = "";
          return;
        }
        const text = art.innerText.toLowerCase();
        if (text.includes(query)) {
          art.style.display = "";
        } else {
          art.style.display = "none";
        }
      });
    });

    // Keyboard shortcut '/' or 'Ctrl+K'
    window.addEventListener("keydown", (e) => {
      if ((e.key === "/" && document.activeElement !== searchInput) || (e.ctrlKey && e.key === "k")) {
        e.preventDefault();
        searchInput.focus();
        searchInput.select();
      }
    });
  }

  // 4. 100 Projects Domain Filter
  const filterBtns = document.querySelectorAll(".filter-btn");
  const projectRows = document.querySelectorAll(".project-item-row");

  filterBtns.forEach((btn) => {
    btn.addEventListener("click", () => {
      filterBtns.forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");

      const filterDomain = btn.getAttribute("data-domain");
      projectRows.forEach((row) => {
        const rowDomain = row.getAttribute("data-domain");
        if (filterDomain === "all" || rowDomain === filterDomain) {
          row.style.display = "";
        } else {
          row.style.display = "none";
        }
      });
    });
  });
});

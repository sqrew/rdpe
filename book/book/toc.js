// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="introduction.html">Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Core Concepts</li><li class="chapter-item expanded "><a href="basics/architecture.html"><strong aria-hidden="true">1.</strong> Architecture Overview</a></li><li class="chapter-item expanded "><a href="basics/particles.html"><strong aria-hidden="true">2.</strong> Particles</a></li><li class="chapter-item expanded "><a href="basics/rules.html"><strong aria-hidden="true">3.</strong> Rules</a></li><li class="chapter-item expanded "><a href="basics/visuals.html"><strong aria-hidden="true">4.</strong> Visual Configuration</a></li><li class="chapter-item expanded "><a href="basics/emitters.html"><strong aria-hidden="true">5.</strong> Emitters</a></li><li class="chapter-item expanded "><a href="basics/typed-interactions.html"><strong aria-hidden="true">6.</strong> Typed Interactions</a></li><li class="chapter-item expanded "><a href="basics/input.html"><strong aria-hidden="true">7.</strong> Input Handling</a></li><li class="chapter-item expanded "><a href="basics/time.html"><strong aria-hidden="true">8.</strong> Time</a></li><li class="chapter-item expanded affix "><li class="part-title">Concepts</li><li class="chapter-item expanded "><a href="concepts/agents.html"><strong aria-hidden="true">9.</strong> Particles as Agents</a></li><li class="chapter-item expanded "><a href="concepts/multi_types.html"><strong aria-hidden="true">10.</strong> Multi-Particle Types</a></li><li class="chapter-item expanded "><a href="concepts/fields.html"><strong aria-hidden="true">11.</strong> Fields</a></li><li class="chapter-item expanded affix "><li class="part-title">Advanced</li><li class="chapter-item expanded "><a href="advanced/spatial-hashing.html"><strong aria-hidden="true">12.</strong> Spatial Hashing</a></li><li class="chapter-item expanded "><a href="advanced/fields.html"><strong aria-hidden="true">13.</strong> 3D Spatial Fields</a></li><li class="chapter-item expanded "><a href="advanced/custom-rules.html"><strong aria-hidden="true">14.</strong> Custom Rules</a></li><li class="chapter-item expanded "><a href="advanced/fragment-shaders.html"><strong aria-hidden="true">15.</strong> Fragment Shaders</a></li><li class="chapter-item expanded "><a href="advanced/textures.html"><strong aria-hidden="true">16.</strong> Textures</a></li><li class="chapter-item expanded "><a href="advanced/post-processing.html"><strong aria-hidden="true">17.</strong> Post-Processing</a></li><li class="chapter-item expanded "><a href="advanced/custom-uniforms.html"><strong aria-hidden="true">18.</strong> Custom Uniforms</a></li><li class="chapter-item expanded "><a href="advanced/custom-functions.html"><strong aria-hidden="true">19.</strong> Custom Functions</a></li><li class="chapter-item expanded "><a href="advanced/shader-utilities.html"><strong aria-hidden="true">20.</strong> Shader Utilities</a></li><li class="chapter-item expanded "><a href="advanced/egui-integration.html"><strong aria-hidden="true">21.</strong> Egui Integration</a></li><li class="chapter-item expanded "><a href="advanced/performance.html"><strong aria-hidden="true">22.</strong> Performance Tips</a></li><li class="chapter-item expanded affix "><li class="part-title">Examples</li><li class="chapter-item expanded "><a href="examples.html"><strong aria-hidden="true">23.</strong> Running Examples</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);

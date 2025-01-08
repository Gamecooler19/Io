// Check if DocsManager is already defined
if (!window.DocsManager) {
    class DocsManager {
        constructor() {
            // Ensure singleton instance
            if (window.DocsManager?.instance) {
                return window.DocsManager.instance;
            }

            // Initialize instance
            this.docsPath = './docs';
            this.currentDoc = null;
            this.cache = new Map();
            this.categories = new Map();
            this.references = {};

            // Store instance
            window.DocsManager.instance = this;

            // Initialize immediately if document is ready, otherwise wait
            if (document.readyState === 'complete' || document.readyState === 'interactive') {
                this.init();
            } else {
                window.addEventListener('DOMContentLoaded', () => this.init());
            }
        }

        async init() {
            // Wait for DOM elements to be ready
            await this.waitForElement('.docs-nav');
            
            try {
                // Try to fetch structure, if fails create default structure
                let structure;
                try {
                    const response = await fetch(`${this.docsPath}/structure.json`);
                    if (!response.ok) throw new Error('Failed to load docs structure');
                    structure = await response.json();
                } catch (error) {
                    console.warn('Creating default docs structure...');
                    structure = await this.createDefaultStructure();
                }

                this.buildNavigation(structure);
                this.setupEventHandlers();
                
                // Load initial doc
                const hash = window.location.hash.replace('#docs/', '');
                const defaultDoc = structure.defaultDoc || 'getting-started/installation';
                await this.loadDoc(hash || defaultDoc);
                
            } catch (error) {
                console.error('Failed to initialize docs:', error);
                this.showError('Documentation is currently unavailable. Please try again later.');
            }
        }

        async waitForElement(selector) {
            return new Promise(resolve => {
                if (document.querySelector(selector)) {
                    return resolve();
                }

                const observer = new MutationObserver(() => {
                    if (document.querySelector(selector)) {
                        observer.disconnect();
                        resolve();
                    }
                });

                observer.observe(document.body, {
                    childList: true,
                    subtree: true
                });
            });
        }

        async createDefaultStructure() {
            // Default structure if docs don't exist
            return {
                defaultDoc: 'getting-started/installation',
                categories: {
                    'getting-started': [
                        {
                            file: 'installation',
                            title: 'Installation'
                        }
                    ]
                }
            };
        }

        buildNavigation(structure) {
            const nav = document.querySelector('.docs-nav');
            nav.innerHTML = ''; // Clear existing navigation
            
            Object.entries(structure.categories).forEach(([category, items]) => {
                const section = document.createElement('div');
                section.className = 'docs-category';
                
                section.innerHTML = `
                    <h4>${this.formatCategoryName(category)}</h4>
                    <ul>
                        ${items.map(item => `
                            <li>
                                <a href="#" data-doc="${category}/${item.file}" 
                                   class="doc-link">
                                    ${item.title}
                                </a>
                            </li>
                        `).join('')}
                    </ul>
                `;
                
                nav.appendChild(section);
            });
        }

        formatCategoryName(name) {
            return name.split('-')
                .map(word => word.charAt(0).toUpperCase() + word.slice(1))
                .join(' ');
        }

        setupEventHandlers() {
            // Setup click handlers for doc links
            document.querySelectorAll('.doc-link').forEach(link => {
                link.addEventListener('click', (e) => {
                    e.preventDefault();
                    const docPath = e.target.getAttribute('data-doc');
                    this.loadDoc(docPath);
                    
                    // Update active state
                    document.querySelectorAll('.doc-link').forEach(a => 
                        a.classList.remove('active'));
                    e.target.classList.add('active');
                    
                    // Update URL
                    history.pushState({docPath}, '', `#docs/${docPath}`);
                });
            });

            // Handle search
            const searchInput = document.getElementById('docsSearch');
            if (searchInput) {
                searchInput.addEventListener('input', (e) => 
                    this.handleSearch(e.target.value));
            }
        }

        async loadDoc(path) {
            const contentDiv = document.getElementById('docsContent');
            contentDiv.classList.add('loading');
            
            try {
                // Check cache first
                let content = this.cache.get(path);
                
                if (!content) {
                    // Use fetch with timeout
                    const controller = new AbortController();
                    const timeout = setTimeout(() => controller.abort(), 5000);
                    
                    try {
                        const response = await fetch(`${this.docsPath}/${path}.md`, {
                            signal: controller.signal
                        });
                        
                        if (!response.ok) {
                            throw new Error(`HTTP error! status: ${response.status}`);
                        }
                        
                        content = await response.text();
                        this.cache.set(path, content);
                        
                    } catch (error) {
                        if (error.name === 'AbortError') {
                            throw new Error('Request timed out');
                        }
                        throw error;
                    } finally {
                        clearTimeout(timeout);
                    }
                }
                
                // Convert markdown to HTML and insert
                const html = this.markdownToHtml(content);
                contentDiv.innerHTML = `<div class="docs-article">${html}</div>`;
                
                // Highlight any code blocks that might have been added dynamically
                Prism.highlightAll();
                
                // Add copy buttons to code blocks
                this.addCodeBlockCopyButtons();
                
            } catch (error) {
                console.error('Documentation loading error:', error);
                this.showError(`Failed to load documentation: ${error.message}`);
            } finally {
                contentDiv.classList.remove('loading');
            }
        }

        addCodeBlockCopyButtons() {
            document.querySelectorAll('.code-block').forEach(block => {
                const copyBtn = block.querySelector('.copy-btn');
                const codeElement = block.querySelector('code');
                
                copyBtn?.addEventListener('click', async () => {
                    try {
                        // Get clean text content without preserving formatting
                        const text = codeElement.textContent
                            .replace(/\u200B/g, '') // Remove zero-width spaces
                            .replace(/\u00A0/g, ' '); // Replace non-breaking spaces
                        
                        await navigator.clipboard.writeText(text);
                        
                        // Update button state
                        const span = copyBtn.querySelector('span');
                        span.textContent = 'Copied!';
                        copyBtn.classList.add('copied');
                        
                        setTimeout(() => {
                            span.textContent = 'Copy';
                            copyBtn.classList.remove('copied');
                        }, 2000);
                    } catch (err) {
                        console.error('Failed to copy:', err);
                        const span = copyBtn.querySelector('span');
                        span.textContent = 'Failed';
                        setTimeout(() => {
                            span.textContent = 'Copy';
                        }, 2000);
                    }
                });
            });
        }

        showError(message) {
            const contentDiv = document.getElementById('docsContent');
            contentDiv.innerHTML = `
                <div class="docs-error">
                    <h3>Error</h3>
                    <p>${message}</p>
                </div>
            `;
        }

        handleSearch(query) {
            query = query.toLowerCase();
            document.querySelectorAll('.doc-link').forEach(link => {
                const text = link.textContent.toLowerCase();
                const docPath = link.getAttribute('data-doc').toLowerCase();
                const visible = text.includes(query) || docPath.includes(query);
                link.parentElement.style.display = visible ? 'block' : 'none';
                
                // Also handle category visibility
                const category = link.closest('.docs-category');
                const hasVisibleItems = Array.from(
                    category.querySelectorAll('.doc-link'))
                    .some(a => a.parentElement.style.display !== 'none');
                category.style.display = hasVisibleItems ? 'block' : 'none';
            });
        }

        markdownToHtml(markdown) {
            // Clean up stray backticks and normalize line endings
            const cleanupMarkdown = (text) => {
                // Remove stray backticks and normalize line endings
                text = text.replace(/^\s*``+\s*$/gm, '');
                text = text.replace(/\r\n/g, '\n');
                text = text.replace(/\n{3,}/g, '\n\n');
                return text;
            };

            // Process code blocks with improved handling
            const processCodeBlocks = (text) => {
                return text.replace(/```([\w-]*)\n([\s\S]*?)```/g, (match, lang, code) => {
                    // Normalize language and handle special cases
                    const language = this.normalizeLanguage(lang.trim());
                    const cleanCode = code.trim();
                    
                    // Highlight code using Prism
                    const highlightedCode = Prism.highlight(
                        cleanCode,
                        Prism.languages[language] || Prism.languages.plaintext,
                        language
                    );

                    // Create formatted code block with proper header and class names
                    return `<div class="code-block ${language}">
                        <div class="code-header">
                            <span class="code-language">${this.formatLanguageName(language)}</span>
                            <div class="code-actions">
                                <button class="copy-btn" title="Copy code">
                                    <svg width="16" height="16" viewBox="0 0 24 24">
                                        <path fill="currentColor" d="M16 1H4C2.9 1 2 1.9 2 3v14h2V3h12V1zm3 4H8C6.9 5 6 5.9 6 7v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
                                    </svg>
                                    <span>Copy</span>
                                </button>
                            </div>
                        </div>
                        <pre class="language-${language}"><code class="language-${language}">${highlightedCode}</code></pre>
                    </div>`;
                });
            };

            // Process inline code with improved handling
            const processInlineCode = (text) => {
                return text.replace(/`([^`]+)`/g, (match, code) => {
                    const cleanCode = code.trim();
                    return `<code class="inline-code">${cleanCode}</code>`;
                });
            };

            // Process markdown in specific order
            let html = cleanupMarkdown(markdown);
            html = html.replace(/^\s*``+\s*(\w+)\s*$/gm, '```$1\n'); // Fix malformed code block starts
            html = processCodeBlocks(html);
            html = this.processHeaders(html);
            html = this.processList(html);
            html = processInlineCode(html);
            html = this.processLinks(html);
            html = this.processBlockquotes(html);
            html = this.processParagraphs(html);

            return html;
        }

        normalizeLanguage(lang = '') {
            const langMap = {
                'sh': 'bash',
                'shell': 'bash',
                'powershell': 'powershell',
                'ps1': 'powershell',
                'ini': 'ini',
                'groovy': 'groovy',
                'cmd': 'batch',
                'batch': 'batch',
                '': 'plaintext'
            };
            return langMap[lang.toLowerCase()] || lang || 'plaintext';
        }

        processHeaders(text) {
            return text
                .replace(/^# (.*$)/gm, '<h1>$1</h1>')
                .replace(/^## (.*$)/gm, '<h2>$1</h2>')
                .replace(/^### (.*$)/gm, '<h3>$1</h3>')
                .replace(/^#### (.*$)/gm, '<h4>$1</h4>')
                .replace(/^##### (.*$)/gm, '<h5>$1</h5>')
                .replace(/^###### (.*$)/gm, '<h6>$1</h6>');
        }

        processList(text) {
            const lines = text.split('\n');
            let inList = false;
            let listType = '';
            
            return lines.map(line => {
                if (line.match(/^\s*[\-\*]\s/)) {
                    // Unordered list
                    if (!inList || listType !== 'ul') {
                        inList = true;
                        listType = 'ul';
                        return '<ul>\n<li>' + line.replace(/^\s*[\-\*]\s/, '') + '</li>';
                    }
                    return '<li>' + line.replace(/^\s*[\-\*]\s/, '') + '</li>';
                } else if (line.match(/^\s*\d+\.\s/)) {
                    // Ordered list
                    if (!inList || listType !== 'ol') {
                        inList = true;
                        listType = 'ol';
                        return '<ol>\n<li>' + line.replace(/^\s*\d+\.\s/, '') + '</li>';
                    }
                    return '<li>' + line.replace(/^\s*\d+\.\s/, '') + '</li>';
                } else if (inList && line.trim() === '') {
                    // End list
                    inList = false;
                    return `</${listType}>\n`;
                } else {
                    if (inList) {
                        inList = false;
                        return `</${listType}>\n${line}`;
                    }
                    return line;
                }
            }).join('\n');
        }

        processLinks(text) {
            return text
                .replace(/\[([^\]]+)\]\(([^\)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
                .replace(/\[([^\]]+)\]\[([^\]]+)\]/g, (match, text, ref) => {
                    const link = this.references[ref.toLowerCase()];
                    return link ? `<a href="${link}">${text}</a>` : match;
                });
        }

        processBlockquotes(text) {
            return text.replace(/^>\s*(.*$)/gm, '<blockquote>$1</blockquote>');
        }

        processParagraphs(text) {
            return text
                .split('\n\n')
                .map(para => {
                    if (
                        !para.startsWith('<h') &&
                        !para.startsWith('<ul') &&
                        !para.startsWith('<ol') &&
                        !para.startsWith('<blockquote') &&
                        !para.startsWith('<pre') &&
                        para.trim()
                    ) {
                        return `<p>${para.trim()}</p>`;
                    }
                    return para;
                })
                .join('\n\n');
        }

        highlightCode(code, language) {
            // Basic syntax highlighting
            if (language === 'iolang') {
                return code
                    .replace(/(fn|let|if|else|return|for|while|struct|impl)/g, '<span class="keyword">$1</span>')
                    .replace(/(".*?")/g, '<span class="string">$1</span>')
                    .replace(/(\/\/.*)/g, '<span class="comment">$1</span>')
                    .replace(/(\d+)/g, '<span class="number">$1</span>');
            }
            // Escape HTML for other languages
            return code.replace(/[&<>"']/g, char => ({
                '&': '&amp;',
                '<': '&lt;',
                '>': '&gt;',
                '"': '&quot;',
                "'": '&#39;'
            })[char]);
        }

        formatLanguageName(lang) {
            // Move this outside of markdownToHtml
            const nameMap = {
                'bash': 'Terminal',
                'shell': 'Terminal',
                'powershell': 'PowerShell',
                'ini': 'Config',
                'plaintext': 'Text',
                'groovy': 'Groovy',
                'javascript': 'JavaScript',
                'python': 'Python'
            };
            return nameMap[lang] || lang.charAt(0).toUpperCase() + lang.slice(1);
        }
    }

    // Initialize singleton only if not already initialized
    if (!window.DocsManager?.instance) {
        window.DocsManager = DocsManager;
        // Wait for document to be ready
        if (document.readyState === 'complete') {
            new DocsManager();
        } else {
            window.addEventListener('load', () => new DocsManager());
        }
    }
}

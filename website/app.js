let currentExample = 0;
const examples = [
    {
        name: 'hello_world.io',
        code: `fn main() {
    println("Welcome to IO Lang Beta!");
    println("What's your name?");
    
    let name = read_line();
    println("Nice to meet you, " + name + "!");
}`
    },
    {
        name: 'calculator.io',
        code: `fn main() {
    println("Simple Calculator");
    println("Enter first number:");
    let a = parse_int(read_line());
    
    println("Enter second number:");
    let b = parse_int(read_line());
    
    println("Choose operation:");
    println("1. Add");
    println("2. Subtract");
    println("3. Multiply");
    println("4. Divide");
    
    let choice = parse_int(read_line());
    let result = 0;
    
    if choice == 1 {
        result = a + b;
        println("Sum = " + to_string(result));
    } else if choice == 2 {
        result = a - b;
        println("Difference = " + to_string(result));
    } else if choice == 3 {
        result = a * b;
        println("Product = " + to_string(result));
    } else if choice == 4 {
        if b != 0 {
            result = a / b;
            println("Quotient = " + to_string(result));
        } else {
            println("Error: Cannot divide by zero!");
        }
    } else {
        println("Invalid choice!");
    }
}`
    }
];

// Initialize global state
let editor;
let isRunning = false;
let terminal;

// Button handling utilities
const handleButtonClick = (button) => {
    const originalContent = button.innerHTML;
    return {
        start: (text) => {
            button.disabled = true;
            button.innerHTML = `
                <svg class="spin" viewBox="0 0 24 24" width="18" height="18">
                    <path fill="currentColor" d="M12 4V2A10 10 0 0 0 2 12h2a8 8 0 0 1 8-8Z"/>
                </svg>
                ${text}
            `;
            return originalContent;
        },
        success: (text = 'Success') => {
            button.disabled = false;
            button.innerHTML = `
                <svg viewBox="0 0 24 24" width="18" height="18">
                    <path fill="currentColor" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/>
                </svg>
                ${text}
            `;
        },
        error: (text = 'Error') => {
            button.disabled = false;
            button.innerHTML = `
                <svg viewBox="0 0 24 24" width="18" height="18">
                    <path fill="currentColor" d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                </svg>
                ${text}
            `;
        },
        reset: () => {
            button.disabled = false;
            button.innerHTML = originalContent;
        }
    };
};

// Terminal utilities
const createTerminalHelpers = (terminal) => ({
    clear: () => {
        terminal.innerHTML = '';
        terminal.classList.remove('terminal-loading');
    },
    append: (text, type = 'output') => {
        const line = document.createElement('div');
        line.className = 'terminal-line';
        line.innerHTML = `
            <span class="terminal-prompt">${type === 'command' ? '$' : '>'}</span>
            <span class="terminal-${type === 'command' ? 'command' : 'output-text'}">${text}</span>
        `;
        terminal.appendChild(line);
        terminal.scrollTop = terminal.scrollHeight;
    }
});

// Initialize application
const initializeApp = () => {
    require(['vs/editor/editor.main'], function() {
        // Make sure the editor container exists before initializing
        const editorContainer = document.getElementById('editor-container');
        if (!editorContainer) {
            console.error('Editor container not found');
            return;
        }

        // Register IO Lang syntax highlighting with Rust-like syntax
        monaco.languages.register({ id: 'iolang' });
        monaco.languages.setMonarchTokensProvider('iolang', {
            keywords: [
                'fn', 'let', 'if', 'else', 'return', 'for', 'in',
                'while', 'break', 'continue', 'struct', 'impl',
                'pub', 'mut', 'match', 'use', 'mod', 'crate',
                'self', 'super', 'where', 'async', 'await', 'type'
            ],
            typeKeywords: [
                'i32', 'i64', 'u32', 'u64', 'f32', 'f64', 
                'bool', 'str', 'String', 'Result', 'Option',
                'Vec', 'Box', 'Rc', 'Arc'
            ],
            operators: [
                '=', '>', '<', '!', '~', '?', ':',
                '==', '<=', '>=', '!=', '&&', '||', '+=', '-=',
                '*=', '/=', '&=', '|=', '^=', '%=', '=>'
            ],
            symbols: /[=><!~?:&|+\-*\/\^%]+/,
            tokenizer: {
                root: [
                    [/[a-zA-Z_]\w*!/, 'macro'],
                    [/[a-zA-Z_]\w*/, {
                        cases: {
                            '@keywords': 'keyword',
                            '@typeKeywords': 'type',
                            '@default': 'identifier'
                        }
                    }],
                    [/".*?"/, 'string'],
                    [/\/\/.*/, 'comment'],
                    [/\d+/, 'number'],
                    [/[{}()\[\]]/, '@brackets'],
                    [/@symbols/, {
                        cases: {
                            '@operators': 'operator',
                            '@default': ''
                        }
                    }]
                ]
            }
        });

        // Enhanced editor settings
        monaco.editor.defineTheme('ioTheme', {
            base: 'vs-dark',
            inherit: true,
            rules: [
                { token: 'keyword', foreground: '#FF79C6', fontStyle: 'bold' },
                { token: 'type', foreground: '#8BE9FD' },
                { token: 'string', foreground: '#F1FA8C' },
                { token: 'number', foreground: '#BD93F9' },
                { token: 'comment', foreground: '#6272A4' },
                { token: 'macro', foreground: '#FFB86C' },
            ],
            colors: {
                'editor.background': '#282A36',
                'editor.foreground': '#F8F8F2',
                'editor.lineHighlightBackground': '#44475A',
                'editor.selectionBackground': '#44475A',
                'editorCursor.foreground': '#F8F8F2',
            }
        });

        // Initialize UI elements
        const buttons = {
            run: document.getElementById('runBtn'),
            share: document.getElementById('shareBtn'),
            format: document.getElementById('formatBtn'),
            settings: document.getElementById('settingsBtn'),
            clear: document.getElementById('clearBtn')
        };

        terminal = document.getElementById('terminal');
        const terminalHelpers = createTerminalHelpers(terminal);

        // Create editor with enhanced settings
        editor = monaco.editor.create(editorContainer, {
            value: examples[0].code,
            language: 'iolang',
            theme: 'ioTheme',
            fontSize: 14,
            fontFamily: 'JetBrains Mono',
            minimap: { enabled: false },
            automaticLayout: true,
            renderWhitespace: 'selection',
            tabSize: 4,
            scrollBeyondLastLine: false,
            padding: { top: 20 },
            smoothScrolling: true,
            cursorSmoothCaretAnimation: true,
            cursorBlinking: 'smooth',
            renderLineHighlight: 'all',
            readOnly: false, // Ensure editor is editable
            contextmenu: true, // Enable right-click menu
            quickSuggestions: true, // Enable code suggestions
            parameterHints: {
                enabled: true
            },
            suggestOnTriggerCharacters: true,
            acceptSuggestionOnEnter: "on"
        });

        window.addEventListener('resize', () => editor.layout());

        // Editor settings management
        const settings = {
            theme: localStorage.getItem('editorTheme') || 'ioTheme',
            fontSize: parseInt(localStorage.getItem('fontSize')) || 14,
            tabSize: parseInt(localStorage.getItem('tabSize')) || 4,
            wordWrap: localStorage.getItem('wordWrap') === 'true' || false,
            minimap: localStorage.getItem('minimap') === 'true' || false
        };

        const updateEditorSettings = (newSettings) => {
            Object.assign(settings, newSettings);
            editor.updateOptions({
                theme: settings.theme,
                fontSize: settings.fontSize,
                tabSize: settings.tabSize,
                wordWrap: settings.wordWrap ? 'on' : 'off',
                minimap: { enabled: settings.minimap }
            });
            
            // Save to localStorage
            Object.entries(settings).forEach(([key, value]) => {
                localStorage.setItem(key, value);
            });
        };

        // Settings panel handlers
        const settingsPanel = document.getElementById('settingsPanel');
        const toggleSettings = () => {
            settingsPanel.classList.toggle('active');
        };

        document.getElementById('closeSettings').addEventListener('click', toggleSettings);
        document.getElementById('editorTheme').addEventListener('change', (e) => {
            updateEditorSettings({ theme: e.target.value });
        });
        document.getElementById('fontSize').addEventListener('change', (e) => {
            updateEditorSettings({ fontSize: parseInt(e.target.value) });
        });
        document.getElementById('tabSize').addEventListener('change', (e) => {
            updateEditorSettings({ tabSize: parseInt(e.target.value) });
        });
        document.getElementById('wordWrap').addEventListener('change', (e) => {
            updateEditorSettings({ wordWrap: e.target.checked });
        });
        document.getElementById('minimap').addEventListener('change', (e) => {
            updateEditorSettings({ minimap: e.target.checked });
        });

        // Documentation search
        const docsSearch = document.getElementById('docsSearch');
        if (docsSearch) {
            docsSearch.addEventListener('input', (e) => {
                const searchTerm = e.target.value.toLowerCase();
                const docLinks = document.querySelectorAll('.docs-category a');
                
                docLinks.forEach(link => {
                    const text = link.textContent.toLowerCase();
                    link.parentElement.style.display = text.includes(searchTerm) ? 'block' : 'none';
                });
            });
        }

        // Copy button functionality
        document.querySelectorAll('.copy-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const code = btn.previousElementSibling.textContent;
                navigator.clipboard.writeText(code);
                
                btn.textContent = 'Copied!';
                setTimeout(() => {
                    btn.textContent = 'Copy';
                }, 2000);
            });
        });

        // Enhanced terminal simulation
        terminal = document.getElementById('terminal');

        const clearTerminal = () => {
            terminal.innerHTML = '';
            terminal.classList.remove('terminal-loading');
        };

        const appendToTerminal = (text, type = 'output') => {
            const line = document.createElement('div');
            line.className = 'terminal-line';
            line.innerHTML = `
                <span class="terminal-prompt">${type === 'command' ? '$' : '>'}</span>
                <span class="terminal-${type === 'command' ? 'command' : 'output-text'}">${text}</span>
            `;
            terminal.appendChild(line);
            terminal.scrollTop = terminal.scrollHeight;
        };

        // Runtime environment simulation
        const IORuntime = {
            variables: new Map(),
            functions: new Map(),
            stack: [],
            currentContext: null,

            createContext() {
                return {
                    variables: new Map(),
                    parent: this.currentContext
                };
            },

            pushContext() {
                this.currentContext = this.createContext();
            },

            popContext() {
                this.currentContext = this.currentContext?.parent || null;
            },

            getVariable(name) {
                let context = this.currentContext;
                while (context) {
                    if (context.variables.has(name)) {
                        return context.variables.get(name);
                    }
                    context = context.parent;
                }
                return this.variables.get(name);
            },

            setVariable(name, value) {
                if (this.currentContext) {
                    this.currentContext.variables.set(name, value);
                } else {
                    this.variables.set(name, value);
                }
            },

            async readLine() {
                return new Promise(resolve => {
                    const inputDiv = document.createElement('div');
                    inputDiv.className = 'terminal-input-line';
                    inputDiv.innerHTML = `
                        <span class="terminal-prompt">$</span>
                        <input type="text" class="terminal-input-field">
                    `;
                    terminal.appendChild(inputDiv);
                    
                    const input = inputDiv.querySelector('input');
                    // Use setTimeout to avoid autofocus issues
                    setTimeout(() => input.focus(), 0);
                    
                    input.addEventListener('keypress', e => {
                        if (e.key === 'Enter') {
                            const value = input.value;
                            inputDiv.innerHTML = `
                                <span class="terminal-prompt">$</span>
                                <span class="terminal-input-text">${value}</span>
                            `;
                            resolve(value);
                        }
                    });
                });
            },

            parseInt(str) {
                const num = Number(str);
                if (isNaN(num)) throw new Error("Invalid number format");
                return num;
            },

            len(arr) {
                if (!Array.isArray(arr)) throw new Error("len() expects an array");
                return arr.length;
            },
            
            toString(value) {
                if (value === undefined) return 'undefined';
                if (value === null) return 'null';
                if (Array.isArray(value)) {
                    return `[${value.join(', ')}]`;
                }
                if (typeof value === 'string') {
                    return value;
                }
                return String(value);
            },

            parseArray(str) {
                try {
                    // Clean up the string and handle empty arrays
                    const cleaned = str.trim().replace(/;$/, '');
                    if (cleaned === '[]') return [];
                    
                    // Verify array format
                    if (!cleaned.startsWith('[') || !cleaned.endsWith(']')) {
                        throw new Error('Invalid array format - must be enclosed in []');
                    }
                    
                    // Parse array content with proper error handling
                    const content = cleaned.slice(1, -1).trim();
                    if (!content) return [];
                    
                    return content.split(',').map(item => {
                        const value = item.trim();
                        
                        // Handle strings
                        if (value.startsWith('"') || value.startsWith("'")) {
                            return value.slice(1, -1);
                        }
                        
                        // Handle numbers
                        const num = Number(value);
                        if (!isNaN(num)) {
                            return num;
                        }
                        
                        // Handle nested arrays
                        if (value.startsWith('[')) {
                            return this.parseArray(value);
                        }
                        
                        return value;
                    });
                } catch (e) {
                    console.error('Array parsing error:', e);
                    throw new Error(`Invalid array format: ${str}`);
                }
            },

            async processArrayDeclaration(line) {
                const match = line.match(/let\s+(\w+)\s*=\s*(\[.+?\])/);
                if (!match) {
                    throw new Error('Invalid array declaration syntax');
                }
        
                const [_, name, arrayStr] = match;
                try {
                    const arrayValue = this.parseArray(arrayStr);
                    this.setVariable(name, arrayValue);
                    return true;
                } catch (e) {
                    console.error('Array declaration error:', e);
                    throw new Error(`Error declaring array ${name}: ${e.message}`);
                }
            },

            async executeFunction(name, args) {
                try {
                    let evaluatedArgs = await Promise.all(args.map(arg => this.evaluateExpression(arg)));
                    
                    switch(name) {
                        case 'println':
                            const text = evaluatedArgs[0];
                            await this.println(text);
                            break;
                        case 'read_line':
                            return await this.readLine();
                        case 'parse_int':
                            return this.parseInt(evaluatedArgs[0]);
                        case 'to_string':
                            const value = evaluatedArgs[0];
                            return this.toString(value);
                        case 'len':
                            const arr = evaluatedArgs[0];
                            if (!Array.isArray(arr)) {
                                throw new Error(`len() expects an array, got ${typeof arr}`);
                            }
                            return arr.length;
                        default:
                            throw new Error(`Unknown function: ${name}`);
                    }
                } catch (error) {
                    console.error('Function execution error:', error);
                    throw error;
                }
            },

            async evaluateExpression(expr) {
                if (!expr) return undefined;
                expr = expr.trim();

                try {
                    // Handle string concatenation first
                    if (expr.includes('+')) {
                        const parts = expr.split('+');
                        const values = await Promise.all(parts.map(async (part) => {
                            const trimmed = part.trim();
                            if (trimmed.startsWith('"') || trimmed.startsWith("'")) {
                                return trimmed.slice(1, -1);
                            }
                            return await this.evaluateExpression(trimmed);
                        }));
                        return values.join('');
                    }

                    // Handle string literals
                    if (expr.startsWith('"') || expr.startsWith("'")) {
                        return expr.slice(1, -1);
                    }

                    // Handle variable references
                    const varValue = this.getVariable(expr);
                    if (varValue !== undefined) {
                        return varValue;
                    }

                    // Handle function calls
                    if (expr.includes('(')) {
                        const funcMatch = expr.match(/(\w+)\((.*)\)/);
                        if (funcMatch) {
                            const [_, funcName, argsStr] = funcMatch;
                            const args = argsStr.split(',').map(arg => arg.trim());
                            return await this.executeFunction(funcName, args);
                        }
                    }

                    return expr;
                } catch (error) {
                    console.error('Expression evaluation error:', error);
                    throw error;
                }
            },

            async println(text) {
                await new Promise(resolve => setTimeout(resolve, 100));
                appendToTerminal(text);
            },

            evaluateString(text) {
                // Remove quotes if present
                text = text.replace(/^["']|["']$/g, '');
                
                // Process string interpolation
                return text.replace(/\{([^}]+)\}/g, (match, expr) => {
                    const value = this.evaluateExpression(expr.trim());
                    return this.toString(value);
                });
            },

            processFunction(name, args) {
                switch(name) {
                    case 'println':
                        return this.println(args[0]);
                    case 'len':
                        return this.len(this.getVariable(args[0]));
                    case 'to_string':
                        return this.toString(this.getVariable(args[0]));
                    case 'parse_int':
                        return this.parseInt(this.getVariable(args[0]));
                    default:
                        throw new Error(`Unknown function: ${name}`);
                }
            },

            async processLine(line) {
                try {
                    // Handle read_line assignments first for proper async handling
                    if (line.includes('read_line()')) {
                        const match = line.match(/let\s+(\w+)\s*=\s*read_line\(\)/);
                        if (match) {
                            const [_, name] = match;
                            const input = await this.readLine();
                            await this.setVariable(name, input);
                            return;
                        }
                    }

                    // Handle println with variable interpolation
                    if (line.includes('println(')) {
                        const match = line.match(/println\((.*)\)/);
                        if (match) {
                            const expr = match[1];
                            const text = await this.evaluateExpression(expr);
                            await this.println(text);
                            return;
                        }
                    }

                    // Handle variable assignments
                    if (line.includes('let')) {
                        const match = line.match(/let\s+(\w+)\s*=\s*(.+)/);
                        if (match) {
                            const [_, name, expression] = match;
                            const value = await this.evaluateExpression(expression);
                            this.setVariable(name, value);
                            return;
                        }
                    }

                    // Handle for loops and other statements
                    const funcMatch = line.match(/(\w+)\((.*)\)/);
                    if (funcMatch) {
                        const [_, funcName, argsStr] = funcMatch;
                        const args = argsStr.split(',').map(arg => arg.trim());
                        await this.executeFunction(funcName, args);
                        return;
                    }

                    throw new Error(`Invalid statement: ${line}`);
                } catch (error) {
                    throw new Error(`Error processing line "${line}": ${error.message}`);
                }
            }
        };

        const executeCode = async (code) => {
            try {
                IORuntime.variables.clear();
                IORuntime.stack = [];
                IORuntime.currentContext = null;
                clearTerminal();

                appendToTerminal('IO Lang Runtime v1.0.0', 'command');
                
                const lines = code.split('\n')
                    .map(line => line.trim())
                    .filter(line => line && !line.startsWith('//'));

                let inFunction = false;
                let blockLevel = 0;
                let skipBlock = false;

                for (let i = 0; i < lines.length; i++) {
                    const line = lines[i];

                    // Handle block level tracking
                    const openBraces = (line.match(/{/g) || []).length;
                    const closeBraces = (line.match(/}/g) || []).length;
                    blockLevel += openBraces;

                    // Handle function declaration
                    if (line.startsWith('fn')) {
                        inFunction = true;
                        IORuntime.pushContext();
                        continue;
                    }

                    // Skip empty lines and lone closing braces
                    if (!line || line === '}') {
                        blockLevel -= closeBraces;
                        if (blockLevel === 0) {
                            inFunction = false;
                            IORuntime.popContext();
                        }
                        continue;
                    }

                    // Skip lines if we're not in the main function
                    if (!inFunction) continue;

                    // Process the line
                    if (!skipBlock) {
                        await IORuntime.processLine(line);
                    }

                    // Update block level after processing
                    blockLevel -= closeBraces;
                }

                appendToTerminal('Program completed successfully', 'success');
            } catch (error) {
                appendToTerminal(`Runtime Error: ${error.message}`, 'error');
                throw error;
            }
        };

        const processLine = async (line) => {
            try {
                // Skip empty lines and block markers
                if (!line || line === '{' || line === '}') {
                    return;
                }

                // Handle array declarations first
                if (line.includes('[')) {
                    return await IORuntime.processArrayDeclaration(line);
                }

                // Handle println statements first
                if (line.includes('println(')) {
                    const match = line.match(/println\((["'])(.*?)\1(?:\s*\+\s*(.+?))?\)/);
                    if (match) {
                        let [_, __, text, expression] = match;
                        if (expression) {
                            const parts = expression.split('+').map(part => {
                                const trimmed = part.trim();
                                if (trimmed.startsWith('"') || trimmed.startsWith("'")) {
                                    return trimmed.slice(1, -1);
                                }
                                if (trimmed.includes('to_string')) {
                                    const varName = trimmed.match(/to_string\((.*?)\)/)[1];
                                    return IORuntime.toString(IORuntime.getVariable(varName));
                                }
                                return IORuntime.toString(IORuntime.getVariable(trimmed));
                            });
                            text += parts.join('');
                        }
                        await IORuntime.println(text);
                        return;
                    }
                }

                // Handle array declarations
                if (line.includes('let') && line.includes('[')) {
                    const match = line.match(/let\s+(\w+)\s*=\s*(.+)/);
                    if (match) {
                        const [_, name, arrayStr] = match;
                        const arrayValue = IORuntime.parseArray(arrayStr);
                        IORuntime.setVariable(name, arrayValue);
                        return;
                    }
                }

                // Handle read_line assignments
                if (line.includes('read_line()')) {
                    const match = line.match(/let\s+(\w+)\s*=\s*read_line\(\)/);
                    if (match) {
                        const [_, name] = match;
                        const input = await IORuntime.readLine();
                        IORuntime.setVariable(name, input);
                        return;
                    }
                }

                // Handle other variable assignments
                if (line.includes('let')) {
                    const match = line.match(/let\s+(\w+)\s*=\s*(.+)/);
                    if (match) {
                        const [_, name, expression] = match;
                        if (expression.includes('parse_int')) {
                            const varName = expression.match(/parse_int\((.*?)\)/)[1];
                            const value = IORuntime.getVariable(varName);
                            IORuntime.setVariable(name, IORuntime.parseInt(value));
                        } else {
                            const value = IORuntime.evaluateExpression(expression);
                            IORuntime.setVariable(name, value);
                        }
                        return;
                    }
                }

                // Handle for loops and other statements
                const funcMatch = line.match(/(\w+)\((.*)\)/);
                if (funcMatch) {
                    const [_, funcName, argsStr] = funcMatch;
                    const args = argsStr.split(',').map(arg => arg.trim());
                    await IORuntime.executeFunction(funcName, args);
                    return;
                }

                throw new Error(`Invalid statement: ${line}`);
            } catch (error) {
                throw new Error(`Error processing line "${line}": ${error.message}`);
            }
        };

        // Improved code execution simulation
        const runCode = async () => {
            if (isRunning) return;
            
            const button = buttons.run;
            const handler = handleButtonClick(button);
            handler.start('Running...');
            
            try {
                isRunning = true;
                terminal.classList.add('terminal-loading');
                clearTerminal();
                
                const code = editor.getValue();
                appendToTerminal('IO Lang Compiler v1.0.0', 'command');
                
                // Simulate compilation steps
                await new Promise(resolve => setTimeout(resolve, 500));
                appendToTerminal('Compiling...', 'output');
                
                await new Promise(resolve => setTimeout(resolve, 800));
                appendToTerminal('Checking types...', 'output');
                
                // Execute code
                await executeCode(code);
                
                button.innerHTML = `
                    <svg viewBox="0 0 24 24" width="18" height="18">
                        <path fill="currentColor" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/>
                    </svg>
                    Success
                `;
            } catch (error) {
                appendToTerminal(`Error: ${error.message}`, 'error');
                button.innerHTML = 'Error';
            } finally {
                isRunning = false;
                terminal.classList.remove('terminal-loading');
                setTimeout(handler.reset, 2000);
            }
        };

        // Example switcher
        const switchExample = () => {
            if (editor.getValue() !== examples[currentExample].code) {
                // Ask for confirmation if code was modified
                if (!confirm('You have unsaved changes. Load example anyway?')) {
                    return;
                }
            }
            currentExample = (currentExample + 1) % examples.length;
            editor.setValue(examples[currentExample].code);
            
            // Update tab name
            const activeTab = document.querySelector('.editor-tab.active');
            activeTab.innerHTML = `
                <svg viewBox="0 0 24 24" width="16" height="16">
                    <path fill="currentColor" d="M14.6 16.6l4.6-4.6-4.6-4.6L16 6l6 6-6 6-1.4-1.4m-5.2 0L4.8 12l4.6-4.6L8 6l-6 6 6 6 1.4-1.4z"/>
                </svg>
                ${examples[currentExample].name}
            `;
        };

        // Improved share functionality
        const shareCode = async () => {
            const button = buttons.share;
            const handler = handleButtonClick(button);
            handler.start('Sharing...');
            
            try {
                const code = editor.getValue();
                const compressed = btoa(encodeURIComponent(code));
                const url = `${window.location.origin}${window.location.pathname}?code=${compressed}`;
                
                await navigator.clipboard.writeText(url);
                button.innerHTML = `
                    <svg viewBox="0 0 24 24" width="18" height="18">
                        <path fill="currentColor" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/>
                    </svg>
                    Copied!
                `;
            } catch (error) {
                console.error('Share error:', error);
                button.innerHTML = 'Error';
            } finally {
                setTimeout(handler.reset, 2000);
            }
        };

        // Unified format code function
        const formatCode = async () => {
            const button = buttons.format;
            const handler = handleButtonClick(button);
            handler.start('Formatting...');
            
            try {
                await new Promise(resolve => setTimeout(resolve, 500));
                const code = editor.getValue();
                const formatted = code
                    .split('\n')
                    .map(line => line.trim())
                    .join('\n')
                    .replace(/\{/g, '{\n    ')
                    .replace(/\}/g, '\n}\n')
                    .replace(/;/g, ';\n    ')
                    .replace(/\n\s*\n/g, '\n\n');  // Remove extra blank lines
                
                editor.setValue(formatted);
                editor.getAction('editor.action.formatDocument').run();
                
                button.innerHTML = `
                    <svg viewBox="0 0 24 24" width="18" height="18">
                        <path fill="currentColor" d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/>
                    </svg>
                    Done
                `;
            } catch (error) {
                console.error('Format error:', error);
                button.innerHTML = 'Error';
            } finally {
                setTimeout(handler.reset, 2000);
            }
        };

        // Button actions
        const actions = {
            run: async () => {
                if (isRunning) return;
                const handler = handleButtonClick(buttons.run);
                try {
                    handler.start('Running...');
                    isRunning = true;
                    terminal.classList.add('terminal-loading');
                    terminalHelpers.clear();
                    await executeCode(editor.getValue());
                    handler.success();
                } catch (error) {
                    terminalHelpers.append(error.message, 'error');
                    handler.error();
                } finally {
                    isRunning = false;
                    terminal.classList.remove('terminal-loading');
                    setTimeout(() => handler.reset(), 2000);
                }
            },
            
            share: async () => {
                const handler = handleButtonClick(buttons.share);
                try {
                    handler.start('Sharing...');
                    const code = editor.getValue();
                    const url = `${location.origin}${location.pathname}?code=${btoa(encodeURIComponent(code))}`;
                    await navigator.clipboard.writeText(url);
                    handler.success('Copied!');
                } catch (error) {
                    handler.error();
                } finally {
                    setTimeout(() => handler.reset(), 2000);
                }
            },

            format: async () => {
                const handler = handleButtonClick(buttons.format);
                try {
                    handler.start('Formatting...');
                    const formatted = await formatCode(editor.getValue());
                    editor.setValue(formatted);
                    handler.success('Done');
                } catch (error) {
                    handler.error();
                } finally {
                    setTimeout(() => handler.reset(), 2000);
                }
            },

            settings: () => {
                const settingsPanel = document.getElementById('settingsPanel');
                settingsPanel.classList.toggle('active');
            },

            clear: () => terminalHelpers.clear()
        };

        // Bind actions to buttons
        Object.entries(actions).forEach(([action, handler]) => {
            buttons[action]?.addEventListener('click', handler);
        });

        // Add keyboard shortcuts
        const shortcuts = {
            'Ctrl+Enter': actions.run,
            'Ctrl+S': actions.share,
            'Ctrl+Shift+F': actions.format,
            'Ctrl+K': actions.settings,
            'Ctrl+L': actions.clear
        };

        Object.entries(shortcuts).forEach(([combo, action]) => {
            const [mod, ...keys] = combo.split('+');
            const keyCode = keys.length > 1 ? 
                monaco.KeyMod.Shift | monaco.KeyCode[`KEY_${keys[1]}`] :
                monaco.KeyCode[`KEY_${keys[0]}`];
            
            editor.addCommand(
                monaco.KeyMod[mod] | keyCode,
                (e) => {
                    if (e) e.preventDefault();
                    action();
                }
            );
        });

        // Initialize with example code if no shared code
        const params = new URLSearchParams(location.search);
        if (params.has('code')) {
            try {
                editor.setValue(decodeURIComponent(atob(params.get('code'))));
            } catch (e) {
                console.error('Failed to load shared code');
            }
        }

        // Error handling
        const showError = (message, line, column) => {
            const errorWidget = document.createElement('div');
            errorWidget.className = 'error-widget';
            errorWidget.textContent = message;
            
            const lineHeight = editor.getOption(monaco.editor.EditorOption.lineHeight);
            const editorLayout = editor.getLayoutInfo();
            
            const position = editor.getScrolledVisiblePosition({ lineNumber: line, column });
            if (position) {
                errorWidget.style.top = `${position.top + lineHeight}px`;
                errorWidget.style.left = `${position.left}px`;
                
                document.getElementById('editor-container').appendChild(errorWidget);
                setTimeout(() => errorWidget.remove(), 3000);
            }
        };

        // Show keyboard shortcuts helper
        let shortcutsTimeout;
        window.addEventListener('keydown', (e) => {
            if (e.ctrlKey || e.metaKey) {
                const shortcutHelp = document.createElement('div');
                shortcutHelp.className = 'keyboard-shortcuts';
                shortcutHelp.innerHTML = Object.entries(shortcuts)
                    .map(([combo, _]) => `
                        <div>
                            <span class="shortcut-key">${combo}</span>
                            ${combo.split('+')[combo.split('+').length - 1]}
                        </div>
                    `).join('');
                
                document.body.appendChild(shortcutHelp);
                
                clearTimeout(shortcutsTimeout);
                shortcutsTimeout = setTimeout(() => {
                    shortcutHelp.remove();
                }, 2000);
            }
        });

        // Initialize docs after editor is ready
        const initializeDocs = () => {
            if (!window.DocsManager?.instance) {
                try {
                    new window.DocsManager();
                } catch (error) {
                    console.error('Failed to initialize docs:', error);
                    setTimeout(initializeDocs, 1000);
                }
            }
        };

        // Check if docs are already initialized
        if (window.DocsManager?.instance) {
            console.log('Docs already initialized');
        } else {
            initializeDocs();
        }

        // Wait for both Monaco and Prism to be ready
        require(['vs/editor/editor.main'], () => {
            // Load docs.js after editor is ready
            const docsScript = document.createElement('script');
            docsScript.src = 'js/docs.js';
            docsScript.onload = initializeDocs;
            document.body.appendChild(docsScript);
        });
    });
};

// Only initialize if we're not already initialized
if (!window.editorInitialized) {
    window.editorInitialized = true;
    initializeApp();
}
import MarkdownIt from 'markdown-it'
// @ts-ignore
import DOMPurify from 'dompurify'
import hljs from 'highlight.js'

// Configure highlight.js
hljs.configure({
    ignoreUnescapedHTML: true
})

const md = new MarkdownIt({
    html: true, // We will sanitize with DOMPurify
    linkify: true,
    typographer: true,
    breaks: true,
    highlight: function (str: string, lang: string): string {
        if (lang && hljs.getLanguage(lang)) {
            try {
                return '<pre class="hljs"><code>' +
                    hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
                    '</code></pre>';
            } catch (__) { }
        }

        return '<pre class="hljs"><code>' + md.utils.escapeHtml(str) + '</code></pre>';
    }
})

// Add safety: all links open in new tab
const defaultRender = md.renderer.rules.link_open || function (tokens: any[], idx: number, options: any, _env: any, self: any) {
    return self.renderToken(tokens, idx, options);
};

md.renderer.rules.link_open = function (tokens: any[], idx: number, options: any, _env: any, self: any) {
    // Add target="_blank"
    tokens[idx].attrSet('target', '_blank');
    // Add rel="noopener noreferrer" for security
    tokens[idx].attrSet('rel', 'noopener noreferrer');

    return defaultRender(tokens, idx, options, _env, self);
};

export const renderMarkdown = (content: string): string => {
    if (!content) return ''
    const rawHtml = md.render(content)
    return DOMPurify.sanitize(rawHtml)
}

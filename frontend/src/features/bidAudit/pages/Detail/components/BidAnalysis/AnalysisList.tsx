import React, { useMemo, useState } from 'react';
import { Tabs, Typography, Tag, Space, Button } from 'antd';
import { CloseOutlined, RightOutlined } from '@ant-design/icons';
import { useStyles } from '../../style';
import { CATEGORY_MAP } from '../../types';
import type { AuditCategory, AuditIssue } from '../../types';
import { useUrlState } from '@/hooks/useUrlState';
import { createPortal } from 'react-dom';

const { Text, Paragraph } = Typography;

type ParsedIssueText = {
   title?: string;
   rationale?: string;
   suggestions: string[];
};

const normalizeAiText = (value: string): string =>
   value
      .replace(/[“”]/g, '"')
      .replace(/[‘’]/g, "'")
      .replace(/\u00A0/g, ' ')
      .trim();

const escapeRegExp = (value: string): string =>
   String(value || '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

const cleanJsonLikeNoise = (value: string): string =>
   normalizeAiText(value)
      .replace(/^\s*\[\s*\{?/, '')
      .replace(/\}?\s*\]\s*$/, '')
      .replace(/"\s*title"\s*:\s*/gi, '')
      .replace(/"\s*severity"\s*:\s*"[^"]*"\s*,?/gi, '')
      .replace(/"\s*rationale"\s*:\s*/gi, '')
      .replace(/"\s*suggestions"\s*:\s*\[[\s\S]*$/gi, '')
      .replace(/[{},[\]]+/g, ' ')
      .replace(/\s{2,}/g, ' ')
      .trim();

const parseIssueText = (raw: string): ParsedIssueText | null => {
   const text = normalizeAiText(raw || '');
   if (!text) return null;

   const tryParse = (value: string): unknown => {
      try {
         return JSON.parse(value);
      } catch {
         return null;
      }
   };

   let parsed = tryParse(text);
   if (!parsed) {
      const start = text.indexOf('[');
      const end = text.lastIndexOf(']');
      if (start >= 0 && end > start) {
         parsed = tryParse(text.slice(start, end + 1));
      }
   }

   const first =
      Array.isArray(parsed) && parsed.length > 0
         ? (parsed[0] as Record<string, unknown>)
         : null;
   if (!first) {
      const markerTitleMatch = text.match(
         /[【\[]\s*问题标题\s*[】\]]\s*[:：]?\s*([\s\S]*?)(?=(?:\n\s*[【\[]\s*问题说明\s*[】\]])|$)/i
      );
      const markerRationaleMatches = Array.from(
         text.matchAll(
            /[【\[]\s*问题说明\s*[】\]]\s*[:：]?\s*([\s\S]*?)(?=(?:\n\s*[【\[]\s*[^\]]+[】\]])|$)/gi
         )
      );
      const titleMatch = text.match(/"title"\s*:\s*"([^"]+)"/i);
      const rationaleMatch = text.match(
         /"rationale"\s*:\s*"([\s\S]*?)"\s*,\s*"suggestions"/i
      );
      const suggestionsBlock = text.match(/"suggestions"\s*:\s*\[([\s\S]*?)\]/i);
      const suggestions = suggestionsBlock
         ? Array.from(suggestionsBlock[1].matchAll(/"([^"]+)"/g)).map((m) => m[1])
         : [];
      const markerTitle = markerTitleMatch?.[1]?.trim();
      const markerRationale = markerRationaleMatches
         .map((match) => String(match[1] || '').trim())
         .filter(Boolean)
         .pop();
      if (
         !titleMatch &&
         !rationaleMatch &&
         suggestions.length === 0 &&
         !markerTitle &&
         !markerRationale
      ) {
         return null;
      }
      return {
         title: titleMatch?.[1] || markerTitle,
         rationale:
            rationaleMatch?.[1] || markerRationale || cleanJsonLikeNoise(text),
         suggestions,
      };
   }

   const suggestions = Array.isArray(first.suggestions)
      ? first.suggestions
           .map((item) => String(item ?? '').trim())
           .filter(Boolean)
      : [];

   return {
      title: typeof first.title === 'string' ? first.title : undefined,
      rationale: typeof first.rationale === 'string' ? first.rationale : undefined,
      suggestions,
   };
};

const splitSuggestionLines = (raw?: string): string[] => {
   if (!raw) return [];
   const normalized = String(raw)
      .replace(/([。！？!?])\s*/g, '$1\n')
      .replace(/[；;]/g, '\n');
   return normalized
      .split(/\n/)
      .map((item) => item.trim())
      .filter(Boolean);
};

const sanitizeSuggestionLines = (lines: string[]): string[] => {
   const invalidPattern = /^[\s.,，。:：;；、\-—_()[\]{}"']*$/;
   const placeholderPattern =
      /^(明确|补充|细化|删除|修订|例如|比如|建议|的时限与标准|。|\.{1,})$/;
   const danglingPattern =
      /(例如[:：]\s*$|第\d+条中\s*$|的表述\s*$|的时限与标准\s*$|与标准[，,:：\s]*例如\s*$)/;
   const trailingVerbPattern = /[，,:：\s]*(明确|补充|完善|细化|增加|定义|说明|约定)\s*$/;
   const splitTailPattern = /(的|与|及|并|且|或|为|是|在|于|对|按|向|从|给|将|需|应|可|应当|必须|以及|并且|并在|,|，|:|：)\s*$/;
   const leadJoinPattern = /^(例如|比如|如|即|并|并且|且|同时|并明确|并约定)/;

   const normalizeLine = (line: string): string => {
      let result = line.replace(/\s+/g, ' ').trim();
      result = result.replace(/^[-•·\d.、\s]+/, '');
      result = result.replace(/^的(表述|时限|标准|要求)?[，,:：\s]*/, '');
      result = result.replace(/^[，,:：。\s]+/, '');
      result = result.replace(/\s*[，,:：]\s*$/, '');
      if (/^避免/.test(result)) {
         result = `修订相关条款，${result}`;
      }
         result = result
            .replace(/\bDocuments?\b/gi, '标书内容')
            .replace(/\bStandards?\b/gi, '政策依据')
            .replace(/\bTopic\b/gi, '审查主题');
      if (result && !/[。！？!?]$/.test(result)) {
         result = `${result}。`;
      }
      return result.trim();
   };

   const normalized = lines
      .map(normalizeLine)
      .filter((line) => line.length > 1)
      .filter((line) => !invalidPattern.test(line));

   const merged: string[] = [];
   for (let i = 0; i < normalized.length; i++) {
      const current = normalized[i];
      const next = normalized[i + 1];
      if (!next) {
         merged.push(current);
         continue;
      }
      const shouldMerge =
         splitTailPattern.test(current) ||
         trailingVerbPattern.test(current) ||
         (current.length <= 14 && leadJoinPattern.test(next));
      if (shouldMerge) {
         merged.push(`${current}${next.replace(/^[，,:：\s]+/, '')}`.trim());
         i += 1;
         continue;
      }
      merged.push(current);
   }

   return Array.from(
      new Set(
         merged
            .filter((line) => !placeholderPattern.test(line))
            .filter((line) => !danglingPattern.test(line))
            .filter((line) => !trailingVerbPattern.test(line))
            .filter((line) => !/^(与|及|并|且|或|和)[\s，,。.:：]/.test(line))
            .filter((line) => !/^(并|且|及|或)$/.test(line))
      )
   );
};

const normalizeIssueTitle = (value?: string): string => {
   const text = String(value || '').trim();
   if (!text) return '';
   if (text === '[]' || text === '[ ]' || text === '【】') return '';
   const cleaned = text
      .replace(/^\s+|\s+$/g, '')
      .replaceAll('[', '')
      .replaceAll(']', '')
      .replaceAll('【', '')
      .replaceAll('】', '')
      .trim();
   if (!cleaned) return '';
   if (/^(null|undefined|none)$/i.test(cleaned)) return '';
   return cleaned;
};

const hasMeaningfulContent = (title: string, rationale?: string): boolean => {
   const normalizedRationale = normalizeIssueTitle(rationale || '');
   if (title) return true;
   if (normalizedRationale && normalizedRationale.length >= 6) return true;
   return false;
};

const isGenericRationale = (value?: string): boolean => {
   const text = normalizeIssueTitle(value || '');
   if (!text) return true;
   if (text.length < 22) return true;
   const genericPatterns = [
      /未在当前文档证据中发现/i,
      /存在遗漏风险/i,
      /可预期性不足/i,
      /不透明/i,
      /需进一步核验/i,
      /建议补充/i,
   ];
   return genericPatterns.some((pattern) => pattern.test(text));
};

const normalizeRationaleForDisplay = (value?: string): string => {
   const text = sanitizeDisplayText(value);
   if (!text) return '';
   let normalized = text;
   const rationaleMarker = '【问题说明】';
   const lastRationaleMarker = normalized.lastIndexOf(rationaleMarker);
   if (lastRationaleMarker >= 0) {
      const tail = normalized
         .slice(lastRationaleMarker + rationaleMarker.length)
         .trim();
      if (tail) {
         normalized = tail;
      }
   }
   normalized = normalized
      .replace(/[【\[]\s*问题标题\s*[】\]]\s*[:：]?[^\n\r]*(?:[\r\n]+|$)/gi, '')
      .replace(/[【\[]\s*问题定位\s*[】\]]\s*第?\s*\d+\s*页?\s*[:：]/gi, '')
      .replace(/[【\[]\s*问题定位\s*[】\]]\s*[:：][\s\S]*?(?=(?:[【\[]\s*问题说明\s*[】\]])|$)/gi, '')
      .replace(/主证据中，?\s*[【\[]\s*问题定位\s*[】\]]/gi, '')
      .replace(/[【\[]\s*问题定位\s*[】\]]/gi, '')
      .trim();
   if (/^【问题说明】/.test(normalized)) {
      normalized = normalized.replace(/^【问题说明】\s*/, '').trim();
   }
   normalized = normalized.replace(/^\s*第?\s*\d+\s*页?\s*[:：]\s*/i, '').trim();
   if (!/[。！？!?]$/.test(normalized)) {
      normalized = `${normalized}。`;
   }
   return normalized;
};

const shouldRenderIssue = (issue: AuditIssue): boolean => {
   const parsed = parseIssueText(issue.description || '');
   const rawDescription = sanitizeDisplayText(issue.description);
   const title = normalizeIssueTitle(parsed?.title || '审查问题');
   const normalizedRationale = normalizeRationaleForDisplay(
      parsed?.rationale || rawDescription
   );
   if (isGenericRationale(normalizedRationale)) {
      const hasLocalAnchor =
         Boolean(parsePageNumber(issue.location?.pageNumber)) ||
         String(issue.location?.context || '').trim().length > 0;
      if (!hasLocalAnchor) {
         return false;
      }
   }
   return hasMeaningfulContent(title, normalizedRationale);
};

const sanitizeDisplayText = (value?: string): string => {
   const text = String(value || '').trim();
   if (!text) return '';
   return text
      .replace(/主证据/g, '审核文件')
      .replace(/\bDocuments?\b/gi, '标书内容')
      .replace(/\bStandards?\b/gi, '政策依据')
      .replace(/\bTopic\b/gi, '审查主题')
      .trim();
};

const parsePageNumber = (value: unknown): number | null => {
   if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
      return Math.floor(value);
   }
   if (typeof value === 'string') {
      const num = Number.parseInt(value, 10);
      if (Number.isFinite(num) && num > 0) {
         return num;
      }
   }
   return null;
};

const normalizeLocatePage = (page: number, sourceFileName?: string): number => {
   void sourceFileName;
   return page;
};

type ParsedSourceRef = {
   fileName: string;
   fileId?: number;
   sourceType?: 'knowledge' | 'tender' | 'unknown';
   previewUrl?: string;
};

const parseSourceReference = (reference?: string): ParsedSourceRef | null => {
   const value = (reference || '').trim();
   if (!value.toLowerCase().startsWith('source://')) {
      return null;
   }
   const match = value.match(/^source:\/\/([^/]+)\/([^/]+)\/(.+)$/i);
   if (!match) {
      return null;
   }
   const sourceTypeRaw = match[1].toLowerCase();
   const sourceType =
      sourceTypeRaw === 'knowledge' || sourceTypeRaw === 'tender'
         ? sourceTypeRaw
         : 'unknown';
   const fileIdNum = Number.parseInt(match[2], 10);
   const fileId = Number.isFinite(fileIdNum) && fileIdNum > 0 ? fileIdNum : undefined;
   let fileName = match[3];
   try {
      fileName = decodeURIComponent(fileName);
   } catch {
      fileName = match[3];
   }
   const normalized = String(fileName || '').replace(/\\/g, '/');
   const nameParts = normalized.split('/').filter(Boolean);
   const baseName = nameParts.length ? nameParts[nameParts.length - 1] : String(fileName || '');
   const previewUrl =
      sourceType === 'knowledge' && fileId
         ? `/api/knowledge-files/${fileId}/preview`
         : fileId
            ? `/api/bid-documents/${fileId}/download`
            : undefined;
   return {
      fileName: baseName || '未返回来源文件',
      fileId,
      sourceType,
      previewUrl,
   };
};

const extractSourceInfo = (
   issue: AuditIssue,
   currentFileName?: string,
   currentFileId?: number
): ParsedSourceRef => {
   void currentFileName;
   void currentFileId;
   const parsedRef = parseSourceReference(issue.reference);
   if (parsedRef && parsedRef.sourceType === 'knowledge') {
      return {
         fileName: '知识库文档',
         fileId: parsedRef.fileId,
         sourceType: 'knowledge',
         previewUrl: parsedRef.previewUrl,
      };
   }
   return {
      fileName: '知识库文档',
      sourceType: 'knowledge',
   };
};

const buildHighlightText = (
   issue: AuditIssue,
   rationale: string,
   title: string
): string => {
   const anchorChars = Array.isArray(issue.anchorCharsRange) ? issue.anchorCharsRange : [];
   const anchorQuote = String(issue.anchorQuote || '').trim();
   if (anchorQuote.length >= 12 && anchorChars.length >= 2) {
      const start = Math.max(0, Number(anchorChars[0]) || 0);
      const end = Math.max(start + 1, Number(anchorChars[1]) || start + 1);
      const mid = Math.floor((start + end) / 2);
      const left = Math.max(0, mid - 20);
      const right = Math.min(anchorQuote.length, left + 40);
      const focused = anchorQuote.slice(left, right).trim();
      if (focused.length >= 8) {
         return focused;
      }
   }
   if (anchorQuote.length >= 6) {
      return anchorQuote.slice(0, 60);
   }
   const context = String(issue.location?.context || '').trim();
   if (context.length >= 6) {
      return context.slice(0, 120);
   }
   const rationaleSentence = String(rationale || '')
      .split(/[。！？!?；;]/)
      .map((s) => s.trim())
      .find((s) => s.length >= 6);
   if (rationaleSentence) {
      return rationaleSentence.slice(0, 120);
   }
   return String(title || '').trim().slice(0, 80);
};

const buildAnchorPrefix = (issue: AuditIssue): string => {
   const quote = String(issue.anchorQuote || issue.location?.context || '').trim();
   if (!quote) {
      return '【问题定位】原文片段待定位';
   }
   const shortQuote = quote.length > 160 ? `${quote.slice(0, 160)}...` : quote;
   return `【问题定位】"${shortQuote}"`;
};

const compactForCompare = (value: string): string =>
   String(value || '')
      .replace(/[\s，,。；;：:!?！？、（）()\[\]【】"“”'‘’]/g, '')
      .trim()
      .toLowerCase();

const removeAnchorOverlapPrefix = (text: string, anchor: string): string => {
   const rawText = String(text || '').trim();
   const rawAnchor = String(anchor || '').trim();
   if (!rawText || !rawAnchor) return rawText;

   const compactText = compactForCompare(rawText);
   const compactAnchor = compactForCompare(rawAnchor);
   if (compactText.length < 8 || compactAnchor.length < 8) return rawText;

   const minOverlap = 6; // Reduced to catch smaller overlaps like "的原则进"
   const maxProbe = Math.min(compactText.length, 160);
   let overlapCompact = '';

   // 1) 优先匹配：文本前缀等于锚点后缀（连续复述场景）
   for (let i = 0; i < compactAnchor.length; i++) {
      const suffix = compactAnchor.slice(i);
      if (suffix.length < minOverlap) continue;
      if (compactText.startsWith(suffix) && suffix.length > overlapCompact.length) {
         overlapCompact = suffix;
      }
   }

   // 2) 兜底匹配：文本前缀在锚点任意位置出现（断句/截断续写场景）
   if (!overlapCompact) {
      for (let len = maxProbe; len >= minOverlap; len--) {
         const prefix = compactText.slice(0, len);
         if (compactAnchor.includes(prefix)) {
            overlapCompact = prefix;
            break;
         }
      }
   }
   if (!overlapCompact) return rawText;

   const ignoreCharPattern = /[\s，,。；;：:!?！？、（）()\[\]【】"“”'‘’]/;
   let pointer = 0;
   let endIndex = -1;
   for (let i = 0; i < rawText.length; i++) {
      const ch = rawText[i];
      if (ignoreCharPattern.test(ch)) {
         continue;
      }
      if (pointer >= overlapCompact.length) {
         endIndex = i;
         break;
      }
      if (ch.toLowerCase() !== overlapCompact[pointer]) {
         return rawText;
      }
      pointer += 1;
      if (pointer === overlapCompact.length) {
         endIndex = i + 1;
         break;
      }
   }
   if (pointer < overlapCompact.length || endIndex < 0) {
      return rawText;
   }
   const stripped = rawText
      .slice(endIndex)
      .replace(/^[\s，,。；;：:!?！？、”"’']+/, '')
      .trim();
   return stripped || rawText;
};

// const trimToAnalyticalStart = (value: string): string => {
//    const text = String(value || '').trim();
//    if (!text) return '';
//    const sentences = text.match(/[^。！？!?；;\n]+[。！？!?；;\n]*/g) || [text];
//    if (sentences.length < 2) return text;

//    const analysisPattern =
//       /(风险|冲突|不一致|不完整|缺失|不明确|可执行|可预期|合规|责任边界|责任分配|预算备案|建议补充|建议明确|建议细化|建议约定|建议统一|应当|需要|需补充|需明确|需细化|可能导致|易引发|不利影响)/;

//    const sentenceLooksAnalytical = (sentence: string): boolean => {
//       const normalized = sentence
//          .replace(/^[“"'【\[]+/, '')
//          .replace(/[”"'】\]]+$/, '')
//          .trim();
//       if (normalized.length < 8) return false;
//       const quoteLikePattern =
//          /(须知前附表|序号\s*条款名称|内容及要求|本表与招标文件|以本表为准|第[一二三四五六七八九十\d]+[章节条款]|采购需求|开标一览表|分项报价表)/;
//       if (quoteLikePattern.test(normalized)) {
//          return false;
//       }
//       return analysisPattern.test(normalized);
//    };

//    let startIndex = -1;
//    for (let i = 0; i < sentences.length; i++) {
//       if (sentenceLooksAnalytical(sentences[i])) {
//          startIndex = i;
//          break;
//       }
//    }
//    if (startIndex <= 0) return text;

//    const prefix = sentences.slice(0, startIndex).join('').trim();
//    if (compactForCompare(prefix).length < 18) return text;
//    const trimmed = sentences.slice(startIndex).join('').trim();
//    return trimmed || text;
// };

// const cutToExplicitAnalyticalClause = (value: string): string => {
//    const text = String(value || '').trim();
//    if (!text) return '';
//    const anchors = [
//       '审核文件仅',
//       '审核文件未',
//       '该条款',
//       '存在',
//       '违反',
//       '建议',
//       '应当',
//       '需要',
//       '需补充',
//       '需明确',
//       '需细化',
//       '可能导致',
//       '易引发',
//    ];
//    let hit = -1;
//    for (const anchor of anchors) {
//       const idx = text.indexOf(anchor);
//       if (idx > 0 && (hit < 0 || idx < hit)) {
//          hit = idx;
//       }
//    }
//    if (hit <= 0) return text;
//    const prefix = text.slice(0, hit).trim();
//    if (compactForCompare(prefix).length < 20) return text;
//    const trimmed = text.slice(hit).replace(/^[，,。；;：:\s]+/, '').trim();
//    return trimmed || text;
// };

const buildAnchorKey = (issue: AuditIssue): string => {
   const quote = String(issue.anchorQuote || issue.location?.context || '').trim();
   const compact = compactForCompare(quote);
   if (compact.length >= 10) return compact.slice(0, 80);
   const fallback = compactForCompare(String(issue.description || '').slice(0, 100));
   return fallback.slice(0, 80);
};

const buildIssueExplanation = (issue: AuditIssue, raw?: string): string => {
   let text = normalizeRationaleForDisplay(raw)
      .replace(/【问题说明】/g, '')
      .replace(/^主证据中，?\s*/g, '')
      .replace(/^审核文件中，?\s*/g, '')
      .trim();

   const anchor = String(issue.anchorQuote || '').trim();
   if (anchor && text) {
      const anchorShort = anchor.slice(0, Math.min(anchor.length, 28));
      const cAnchor = compactForCompare(anchorShort);
      const cText = compactForCompare(text);
      if (cAnchor && cText.startsWith(cAnchor)) {
         text = text
            .replace(/^[“"][^”"]{4,260}[”"]\s*[，,。；;:：]?\s*/, '')
            .trim();
         if (compactForCompare(text).startsWith(cAnchor)) {
            text = text
               .replace(
                  new RegExp(
                     `^${escapeRegExp(anchorShort)}[\\s，,。；;:：-]*`,
                     'i'
                  ),
                  ''
               )
               .trim();
         }
      }
   }
   if (anchor && text) {
      text = removeAnchorOverlapPrefix(text, anchor);
   }

   if (!text) {
      text = '该条款与合同执行或合规要求不一致，需要按证据片段补充可执行约束与责任条款。';
   }
   if (!/[。！？!?]$/.test(text)) {
      text = `${text}。`;
   }
   return text;
};

interface AnalysisListProps {
   issues: AuditIssue[];
   isComplete: boolean;
   onLocateIssuePage: (page: number, highlightText?: string, fallbackTokens?: string[]) => void;
   overlayHost?: HTMLElement | null;
   currentFileName?: string;
   currentFileId?: number;
}

export const AnalysisList: React.FC<AnalysisListProps> = React.memo(
   ({ issues, isComplete, onLocateIssuePage, overlayHost, currentFileName, currentFileId }) => {
      const { theme } = useStyles();
      const [queryParams, setQueryParams] = useUrlState({ tab: 'all' });
      const currentTab = queryParams.tab;
      const [activeCategory, setActiveCategory] = useState<AuditCategory | null>(
         null
      );
      const visibleIssues = useMemo(
         () => (issues || []).filter((i) => i && shouldRenderIssue(i)),
         [issues]
      );
      const visibleSummary = useMemo(
         () => ({
            critical: visibleIssues.filter((i) => i?.severity === 'critical').length,
            warning: visibleIssues.filter((i) => i?.severity === 'warning').length,
            info: visibleIssues.filter((i) => i?.severity === 'info').length,
         }),
         [visibleIssues]
      );

      const filteredIssues = useMemo(() => {
         if (currentTab === 'critical')
            return visibleIssues.filter((i) => i?.severity === 'critical');
         if (currentTab === 'warning')
            return visibleIssues.filter((i) => i?.severity === 'warning');
         if (currentTab === 'info')
            return visibleIssues.filter((i) => i?.severity === 'info');
         return visibleIssues;
      }, [visibleIssues, currentTab]);

      const canonicalPageByAnchor = useMemo(() => {
         const pageVotes = new Map<string, Map<number, number>>();
         visibleIssues.forEach((issue) => {
            const page =
               parsePageNumber(issue.anchorPage) ||
               parsePageNumber(issue.location?.pageNumber);
            if (!page) return;
            const key = buildAnchorKey(issue);
            if (!key) return;
            const votes = pageVotes.get(key) || new Map<number, number>();
            votes.set(page, (votes.get(page) || 0) + 1);
            pageVotes.set(key, votes);
         });
         const result = new Map<string, number>();
         pageVotes.forEach((votes, key) => {
            let bestPage = 0;
            let bestCount = -1;
            votes.forEach((count, page) => {
               if (count > bestCount || (count === bestCount && page > 0 && page < bestPage)) {
                  bestCount = count;
                  bestPage = page;
               }
            });
            if (bestPage > 0) result.set(key, bestPage);
         });
         return result;
      }, [visibleIssues]);

      const tabItems = useMemo(
         () => [
            { key: 'all', label: `全部 (${visibleIssues.length})` },
            { key: 'critical', label: `严重 (${visibleSummary.critical})` },
            { key: 'warning', label: `一般 (${visibleSummary.warning})` },
            { key: 'info', label: `提示 (${visibleSummary.info})` },
         ],
         [visibleIssues.length, visibleSummary]
      );

      const categoryPanels = useMemo(() => {
         return (Object.keys(CATEGORY_MAP) as AuditCategory[]).map((categoryKey) => {
               const categoryIssues = filteredIssues
                  .filter(Boolean)
                  .filter((i) => (i?.category || i?.dimension) === categoryKey);
               const stretchCard = categoryIssues.length === 1;
               const renderedIssues = categoryIssues
                  .map((issue, issueIndex) => {
                     const parsed = parseIssueText(issue.description);
                     const rawDescription = sanitizeDisplayText(issue.description);
                     const title = normalizeIssueTitle(
                        parsed?.title ||
                           (rawDescription.length > 0 && rawDescription.length <= 36
                              ? rawDescription
                              : '审查问题')
                     );
                     const rationaleBody = buildIssueExplanation(
                        issue,
                        parsed?.rationale || rawDescription
                     );
                     const rationale = `${buildAnchorPrefix(issue)}\n【问题说明】${rationaleBody}`;
                     if (!hasMeaningfulContent(title, rationale)) {
                        return null;
                     }
                     const suggestionSource =
                        parsed?.suggestions && parsed.suggestions.length > 0
                           ? parsed.suggestions
                           : splitSuggestionLines(issue.suggestion);
                     const suggestionLines =
                        sanitizeSuggestionLines(suggestionSource);
                     const sourceInfo = extractSourceInfo(issue, currentFileName, currentFileId);

                     const issueAnchorKey = buildAnchorKey(issue);
                     const canonicalPage =
                        (issueAnchorKey ? canonicalPageByAnchor.get(issueAnchorKey) : undefined) || null;
                     const rawPageNo =
                        canonicalPage ||
                        parsePageNumber(issue.anchorPage) ||
                        parsePageNumber(issue.location?.pageNumber);
                     const pageNo = rawPageNo
                        ? normalizeLocatePage(rawPageNo, sourceInfo.fileName)
                        : null;
                     const issueRenderKey = `${issue.issueNo || 'issue'}-${rawPageNo}-${issueIndex}`;

                     return (
                        <div
                           key={issueRenderKey}
                           style={{
                              flex: stretchCard ? 1 : undefined,
                              paddingBottom: 8,
                              borderBottom: `1px dashed ${theme.colorBorderSecondary}`,
                              borderLeft: `4px solid ${
                                 issue.severity === 'critical'
                                    ? theme.colorError
                                    : issue.severity === 'warning'
                                    ? theme.colorWarning
                                    : theme.colorPrimary
                              }`,
                              paddingLeft: 12,
                           }}
                        >
                           <Space style={{ marginBottom: 8 }}>
                              <Tag
                                 style={{ fontSize: '1.15rem' }}
                                 color={
                                    issue.severity === 'critical'
                                       ? 'error'
                                       : issue.severity === 'warning'
                                       ? 'warning'
                                       : 'processing'
                                 }
                              >
                                 <span
                                    onClick={() => {
                                       const page =
                                          canonicalPage ||
                                          parsePageNumber(issue.anchorPage) ||
                                          parsePageNumber(issue.location?.pageNumber);
                                       if (page) {
                                          const highlightText = buildHighlightText(
                                             issue,
                                             rationale,
                                             title
                                          );
                                          const fallbackTokens = Array.isArray(issue.anchorTokens)
                                             ? issue.anchorTokens
                                                  .map((item) => String(item || '').trim())
                                                  .filter(Boolean)
                                                  .slice(0, 5)
                                             : [];
                                          onLocateIssuePage(
                                             normalizeLocatePage(page, sourceInfo.fileName),
                                             highlightText,
                                             fallbackTokens
                                          );
                                       }
                                    }}
                                    style={{ cursor: pageNo ? 'pointer' : 'default' }}
                                 >
                                    {pageNo ? `第 ${pageNo} 页` : '页码待定位'}
                                 </span>
                              </Tag>

                              {title ? (
                                 <Text
                                    strong
                                    style={{
                                       fontSize: '1.15rem',
                                       letterSpacing: '0.8px',
                                    }}
                                 >
                                    {title}
                                 </Text>
                              ) : null}
                           </Space>
                           {rationale && (
                              <Paragraph
                                 style={{
                                    marginBottom: 6,
                                    color: '#262626',
                                    fontWeight: 500,
                                    fontSize: '1.25rem',
                                    lineHeight: 1.85,
                                    whiteSpace: 'pre-wrap',
                                    fontFamily:
                                       '"Times New Roman","Noto Serif SC","Songti SC",serif',
                                 }}
                              >
                                 {rationale}
                              </Paragraph>
                           )}
                           {suggestionLines.length > 0 && (
                              <ul
                                 style={{
                                    margin: 0,
                                    paddingLeft: 20,
                                    color: theme.colorTextSecondary,
                                    fontSize: '1.15rem',
                                    lineHeight: 1.9,
                                    fontWeight: 700,
                                    fontFamily:
                                       '"PingFang SC","Microsoft YaHei","Noto Sans SC",sans-serif',
                                 }}
                              >
                                 {suggestionLines.map((line, lineIndex) => (
                                    <li key={`${issueRenderKey}-${lineIndex}`}>
                                       {line}
                                    </li>
                                 ))}
                              </ul>
                           )}
                           <div
                              style={{
                                 marginTop: 8,
                                 padding: '6px 10px',
                                 borderRadius: 6,
                                 background: '#f0f7ff',
                                 border: '1px solid #91caff',
                              }}
                           >
                              <Text
                                 style={{
                                    fontSize: '0.95rem',
                                    fontWeight: 700,
                                    color: '#0958d9',
                                    marginRight: 8,
                                 }}
                              >
                                 RAG引用文件
                              </Text>
                              <Text
                                 style={{
                                    fontSize: '1rem',
                                    color: '#1d39c4',
                                 }}
                              >
                                 {sourceInfo.previewUrl ? (
                                    <a
                                       href={sourceInfo.previewUrl}
                                       target='_blank'
                                       rel='noreferrer'
                                       style={{ color: '#1d39c4' }}
                                    >
                                       {sourceInfo.fileName}
                                    </a>
                                 ) : (
                                    sourceInfo.fileName
                                 )}
                              </Text>
                           </div>
                        </div>
                     );
                  })
                  .filter(Boolean);

               return { key: categoryKey, renderedIssues };
            });
      }, [filteredIssues, theme, onLocateIssuePage, currentFileName, currentFileId, canonicalPageByAnchor]);

      const currentPanel = useMemo(
         () =>
            activeCategory
               ? categoryPanels.find((item) => item.key === activeCategory) || null
               : null,
         [activeCategory, categoryPanels]
      );

      return (
         <div
            style={{
               flex: 1,
               display: 'flex',
               flexDirection: 'column',
            }}
         >
            <Tabs
               activeKey={currentTab}
               onChange={(key) => setQueryParams({ tab: key })}
               items={tabItems}
               style={{ paddingLeft: 6, height: 'auto', flex: 'none' }}
               size='small'
            />

            <div
               style={{
                  flex: 1,
                  minHeight: 0,
                  overflowY: 'auto',
                  scrollbarWidth: 'none',
                  msOverflowStyle: 'none',
               }}
            >
               <div
                  style={{
                     display: 'flex',
                     flexDirection: 'column',
                     gap: 8,
                     paddingRight: 4,
                  }}
               >
                  {categoryPanels.map((panel) => (
                     <div
                        key={panel.key}
                        onClick={() => setActiveCategory(panel.key)}
                        style={{
                           display: 'flex',
                           alignItems: 'center',
                           justifyContent: 'space-between',
                           cursor: 'pointer',
                           padding: '12px 14px',
                           border: `1px solid ${theme.colorBorderSecondary}`,
                           borderRadius: 8,
                           background: theme.colorBgContainer,
                        }}
                     >
                        <Space>
                           <Text strong style={{ fontSize: '1.25rem' }}>
                              {CATEGORY_MAP[panel.key]} ({panel.renderedIssues.length})
                           </Text>
                           {panel.renderedIssues.length === 0 && isComplete && (
                              <Tag color='success'>
                                 无异常
                              </Tag>
                           )}
                        </Space>
                        <RightOutlined />
                     </div>
                  ))}
               </div>
            </div>

            {currentPanel &&
               (() => {
                  const overlayNode = (
                     <div
                        style={{
                           position: 'absolute',
                           inset: 0,
                           zIndex: 200,
                           background: theme.colorBgContainer,
                           borderRadius: 8,
                           border: `1px solid ${theme.colorBorderSecondary}`,
                           padding: 16,
                           display: 'flex',
                           flexDirection: 'column',
                           gap: 10,
                        }}
                     >
                        <div
                           style={{
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'space-between',
                           }}
                        >
                           <Text strong style={{ fontSize: '1.25rem' }}>
                              {CATEGORY_MAP[currentPanel.key]} ({currentPanel.renderedIssues.length})
                           </Text>
                           <Button
                              type='text'
                              icon={<CloseOutlined />}
                              onClick={() => setActiveCategory(null)}
                           />
                        </div>
                        <div
                           style={{
                              overflowY: 'auto',
                              flex: 1,
                              paddingRight: 4,
                              scrollbarWidth: 'none',
                              msOverflowStyle: 'none',
                           }}
                        >
                           {currentPanel.renderedIssues.length === 0 ? (
                              <Text
                                 type='secondary'
                                 style={{ paddingLeft: 12, fontSize: '1.15rem' }}
                              >
                                 暂未发现相关问题
                              </Text>
                           ) : (
                              <div
                                 style={{
                                    display: 'flex',
                                    flexDirection: 'column',
                                    gap: 12,
                                 }}
                              >
                                 {currentPanel.renderedIssues}
                              </div>
                           )}
                        </div>
                     </div>
                  );
                  if (overlayHost) {
                     return createPortal(overlayNode, overlayHost);
                  }
                  return overlayNode;
               })()}
         </div>
      );
   }
);

import React from 'react';
import { Typography, Tooltip, theme } from 'antd';
import {
   RobotOutlined,
   UserOutlined,
   ExclamationCircleOutlined,
} from '@ant-design/icons';
import type { ChatMessage } from '../../hooks/useAiChat';

const { Text, Paragraph, Link } = Typography;

type StructuredIssue = {
   title?: string;
   severity?: string;
   rationale?: string;
   suggestions?: string[];
};

type SavedSummaryBlock = {
   title: string;
   point: string;
};

type SourceRef = {
   key: string;
   fileId?: number;
   fileName: string;
   sourceType?: string;
   pageNumber?: number;
   sectionName?: string;
   previewUrl?: string;
};

const severityLabel: Record<string, string> = {
   critical: '严重',
   warning: '警告',
   info: '提示',
};

const extractTailSuggestions = (content: string): string[] => {
   const tail = content.slice(Math.max(content.lastIndexOf(']') + 1, 0)).trim();
   if (!tail) return [];
   const parts = tail
      .replace(/\s+/g, ' ')
      .split(/(?=细化|绑定|约定|明确|补充|增加|删除)/)
      .map((item) => item.replace(/^[,，;；\s]+/, '').trim())
      .filter(Boolean);
   return parts.filter((item) => item.length >= 6);
};

const parseLooseIssues = (content: string): StructuredIssue[] | null => {
   const titlePattern = /"title"\s*:\s*"([^"]+)"/g;
   const matches = Array.from(content.matchAll(titlePattern));
   if (matches.length === 0) return null;

   const issues: StructuredIssue[] = [];
   for (let i = 0; i < matches.length; i++) {
      const start = matches[i].index ?? 0;
      const end =
         i + 1 < matches.length ? (matches[i + 1].index ?? content.length) : content.length;
      const block = content.slice(start, end);

      const titleMatch = block.match(/"title"\s*:\s*"([^"]+)"/);
      const severityMatch = block.match(/"severity"\s*:\s*"([^"]+)"/);
      const rationaleMatch = block.match(/"rationale"\s*:\s*"([\s\S]*?)"(?:\s*,\s*"|$)/);
      const suggestionsBlockMatch = block.match(/"suggestions"\s*:\s*\[([\s\S]*?)\]/);
      const suggestions = suggestionsBlockMatch
         ? Array.from(suggestionsBlockMatch[1].matchAll(/"([^"]+)"/g)).map((m) => m[1])
         : [];

      issues.push({
         title: titleMatch?.[1],
         severity: severityMatch?.[1],
         rationale: rationaleMatch?.[1],
         suggestions,
      });
   }

   const tailSuggestions = extractTailSuggestions(content);
   if (tailSuggestions.length > 0 && issues.length > 0) {
      const last = issues[issues.length - 1];
      last.suggestions = [...(last.suggestions || []), ...tailSuggestions];
   }

   return issues;
};

const parseStructuredIssues = (content: string): StructuredIssue[] | null => {
   const trimmed = content.trim();
   const tryParse = (raw: string): unknown => {
      try {
         return JSON.parse(raw);
      } catch {
         return null;
      }
   };

   const direct = tryParse(trimmed);
   if (Array.isArray(direct)) return direct as StructuredIssue[];

   const start = trimmed.indexOf('[');
   const end = trimmed.lastIndexOf(']');
   if (start >= 0 && end > start) {
      const sliced = tryParse(trimmed.slice(start, end + 1));
      if (Array.isArray(sliced)) return sliced as StructuredIssue[];
   }
   return parseLooseIssues(trimmed);
};

const stripTrailingJsonNoise = (content: string): string => {
   let cleaned = content;
   cleaned = cleaned.replace(/["“”]?citations["“”]?\s*:\s*\[[\s\S]*$/i, '');
   cleaned = cleaned.replace(/["“”]meta["“”]?\s*:\s*\{[\s\S]*$/i, '');
   cleaned = cleaned.replace(/[,\s]+$/, '').trim();
   return cleaned;
};

const prettifyRawContent = (content: string): string => {
   return stripTrailingJsonNoise(content)
      .replace(/},\s*{/g, '}\n\n{')
      .replace(/"title"\s*:\s*/g, '\n标题：')
      .replace(/"severity"\s*:\s*/g, '\n级别：')
      .replace(/"rationale"\s*:\s*/g, '\n依据：')
      .replace(/"suggestions"\s*:\s*\[/g, '\n建议：\n[')
      .replace(/\],\s*{/g, ']\n\n{')
      .replace(/"\s*,\s*"/g, '"\n"')
      .replace(/\s{2,}/g, ' ')
      .trim();
};

const parseSavedSummary = (content: string): SavedSummaryBlock[] | null => {
   const prefix = '已保存记录，归纳内容如下：';
   const text = String(content || '').trim();
   if (!text.startsWith(prefix)) {
      return null;
   }
   const body = text
      .slice(prefix.length)
      .trim()
      .replace(/\\n/g, '\n');
   if (!body) {
      return [];
   }
   const blockRegex = /【([^】]+)】\s*\n(?:- 要点：)?([\s\S]*?)(?=\n\s*【|$)/g;
   const blocks: SavedSummaryBlock[] = [];
   for (const match of body.matchAll(blockRegex)) {
      const title = (match[1] || '').trim();
      const point = (match[2] || '').trim().replace(/\n+/g, '\n');
      if (!title || !point) continue;
      blocks.push({ title, point });
   }
   return blocks;
};

const toPositiveInt = (value: unknown): number | undefined => {
   if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
      return Math.floor(value);
   }
   if (typeof value === 'string') {
      const parsed = Number.parseInt(value, 10);
      if (Number.isFinite(parsed) && parsed > 0) {
         return parsed;
      }
   }
   return undefined;
};

const getSourceRefs = (message: ChatMessage): SourceRef[] => {
   if (!Array.isArray(message.citations) || message.citations.length === 0) {
      return [];
   }
   const refs: SourceRef[] = [];
   const dedupe = new Set<string>();
   for (const citation of message.citations) {
      const meta = citation?.meta ?? {};
      const fileId = toPositiveInt(meta.fileId ?? meta.file_id ?? meta.document_id);
      const sourceTypeRaw = String(meta.sourceType ?? meta.source_type ?? '').toLowerCase();
      const sourceType = sourceTypeRaw === 'knowledge' ? sourceTypeRaw : undefined;
      if (!sourceType) {
         continue;
      }
      const fileName =
         String(meta.fileName ?? meta.file_name ?? citation.documentName ?? '').trim() ||
         (fileId ? `文件#${fileId}` : '未知文件');
      const pageNumber = toPositiveInt(meta.pageNumber ?? citation.pageNumber);
      const sectionName =
         typeof meta.sectionName === 'string' && meta.sectionName.trim()
            ? meta.sectionName.trim()
            : undefined;
      const previewUrl =
         fileId && sourceType === 'knowledge'
            ? `/api/knowledge-files/${fileId}/preview`
            : undefined;
      const key = `${sourceType || 'unknown'}-${fileId || fileName}-${pageNumber || 'na'}`;
      if (dedupe.has(key)) {
         continue;
      }
      dedupe.add(key);
      refs.push({
         key,
         fileId,
         fileName,
         sourceType,
         pageNumber,
         sectionName,
         previewUrl,
      });
   }
   return refs;
};

export const MessageBubble: React.FC<{ message: ChatMessage }> = React.memo(
   ({ message }) => {
      const { token } = theme.useToken();
      const isUser = message.role === 'user';
      const sourceRefs = !isUser ? getSourceRefs(message) : [];

      return (
         <div
            style={{
               display: 'flex',
               flexDirection: isUser ? 'row-reverse' : 'row',
               gap: 8,
               marginBottom: 14,
               alignItems: 'flex-start',
            }}
         >
            <div
               style={{
                  width: 30,
                  height: 30,
                  borderRadius: '50%',
                  background: isUser
                     ? token.colorFillAlter
                     : token.colorPrimary,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  flexShrink: 0,
               }}
            >
               {isUser ? (
                  <UserOutlined
                     style={{
                        color: token.colorPrimary,
                        fontSize: 13,
                     }}
                  />
               ) : (
                  <RobotOutlined
                     style={{
                        color: token.colorTextLightSolid || '#fff',
                        fontSize: 13,
                     }}
                  />
               )}
            </div>

            <div
               style={{
                  maxWidth: '76%',
                  padding: '8px 12px',
                  borderRadius: isUser
                     ? '12px 2px 12px 12px'
                     : '2px 12px 12px 12px',
                  background: isUser
                     ? token.colorFillAlter
                     : token.colorBgLayout,
                  wordBreak: 'break-word',
                  position: 'relative',
                  border: `1px solid ${token.colorBorderSecondary}`,
               }}
            >
               {(() => {
                  const savedBlocks =
                     !isUser && message.content ? parseSavedSummary(message.content) : null;
                  if (savedBlocks && savedBlocks.length > 0) {
                     return (
                        <div style={{ display: 'grid', gap: 8 }}>
                           <Text strong style={{ fontSize: 13 }}>
                              已保存记录（归纳）
                           </Text>
                           {savedBlocks.map((item, idx) => (
                              <div
                                 key={`${item.title}-${idx}`}
                                 style={{
                                    border: `1px solid ${token.colorBorderSecondary}`,
                                    borderRadius: 8,
                                    padding: '8px 10px',
                                    background: token.colorBgContainer,
                                 }}
                              >
                                 <Text strong style={{ fontSize: 13 }}>
                                    {idx + 1}. {item.title}
                                 </Text>
                                 <Paragraph
                                    style={{
                                       margin: '6px 0 0',
                                       fontSize: 13,
                                       lineHeight: 1.6,
                                       whiteSpace: 'pre-wrap',
                                    }}
                                 >
                                    {item.point}
                                 </Paragraph>
                              </div>
                           ))}
                        </div>
                     );
                  }

                  const issues =
                     !isUser && message.content
                        ? parseStructuredIssues(message.content)
                        : null;

                  if (issues && issues.length > 0) {
                     return (
                        <div style={{ display: 'grid', gap: 10 }}>
                           {issues.map((item, idx) => (
                              <div
                                 key={`${item.title ?? 'item'}-${idx}`}
                                 style={{
                                    border: `1px solid ${token.colorBorderSecondary}`,
                                    borderRadius: 8,
                                    padding: '8px 10px',
                                    background: token.colorBgContainer,
                                 }}
                              >
                                 <Text strong style={{ fontSize: 13 }}>
                                    {idx + 1}. {item.title || '未命名问题'}
                                 </Text>
                                 <Text
                                    style={{
                                       marginLeft: 8,
                                       fontSize: 12,
                                       color: token.colorTextSecondary,
                                    }}
                                 >
                                    {severityLabel[item.severity || ''] ||
                                       item.severity ||
                                       '未知级别'}
                                 </Text>
                                 {item.rationale && (
                                    <Paragraph
                                       style={{
                                          margin: '6px 0 0',
                                          fontSize: 13,
                                          lineHeight: 1.6,
                                          whiteSpace: 'pre-wrap',
                                       }}
                                    >
                                       {item.rationale}
                                    </Paragraph>
                                 )}
                                 {Array.isArray(item.suggestions) &&
                                    item.suggestions.length > 0 && (
                                       <ul
                                          style={{
                                             margin: '6px 0 0',
                                             paddingLeft: 18,
                                          }}
                                       >
                                          {item.suggestions
                                             .filter(
                                                (s) =>
                                                   !!s &&
                                                   s.trim().length > 1 &&
                                                   !/^[,，。.:：;；\s-]+$/.test(s)
                                             )
                                             .map((s, i) => (
                                             <li
                                                key={`${idx}-${i}`}
                                                style={{
                                                   fontSize: 13,
                                                   lineHeight: 1.6,
                                                }}
                                             >
                                                {s}
                                             </li>
                                          ))}
                                       </ul>
                                    )}
                              </div>
                           ))}
                        </div>
                     );
                  }

                  return (
                     <Paragraph
                        style={{
                           margin: 0,
                           fontSize: 13,
                           lineHeight: 1.65,
                           whiteSpace: 'pre-wrap',
                           color: token.colorTextBase,
                        }}
                     >
                        {isUser
                           ? message.content
                           : prettifyRawContent(message.content)}
                     </Paragraph>
                  );
               })()}
               {!isUser && sourceRefs.length > 0 && (
                  <div
                     style={{
                        marginTop: 8,
                        paddingTop: 6,
                        borderTop: `1px dashed ${token.colorBorderSecondary}`,
                        display: 'grid',
                        gap: 4,
                     }}
                  >
                     <Text
                        style={{
                           fontSize: 12,
                           color: token.colorTextSecondary,
                        }}
                     >
                        引用来源
                     </Text>
                     <div
                        style={{
                           display: 'flex',
                           flexWrap: 'wrap',
                           gap: 8,
                        }}
                     >
                        {sourceRefs.map((ref) => (
                           <Link
                              key={ref.key}
                              disabled={!ref.previewUrl}
                              onClick={(event) => {
                                 if (!ref.previewUrl) {
                                    return;
                                 }
                                 event.preventDefault();
                                 window.open(ref.previewUrl, '_blank', 'noopener,noreferrer');
                              }}
                           >
                              {ref.fileName}
                              {ref.pageNumber ? `（第${ref.pageNumber}页）` : ''}
                           </Link>
                        ))}
                     </div>
                  </div>
               )}

               {message.status === 'error' && (
                  <Tooltip title='Send failed – please retry'>
                     <ExclamationCircleOutlined
                        style={{
                           color: token.colorError,
                           position: 'absolute',
                           right: -20,
                           top: 8,
                           cursor: 'pointer',
                        }}
                     />
                  </Tooltip>
               )}

               <Text
                  style={{
                     fontSize: 10,
                     color: '#bfbfbf',
                     display: 'block',
                     marginTop: 4,
                     textAlign: isUser ? 'right' : 'left',
                  }}
               >
                  {new Date(message.createTime).toLocaleTimeString([], {
                     hour: '2-digit',
                     minute: '2-digit',
                  })}
               </Text>
            </div>
         </div>
      );
   }
);

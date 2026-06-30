import React from 'react';
import { Typography, Space } from 'antd';
import { LinkOutlined } from '@ant-design/icons';
import type { Citation } from '@/types/audit';

const { Link } = Typography;

interface CitationListProps {
  citations?: Citation[];
}

/**
 * 搜索来源引用列表 — 对齐 Rust Citation。
 * 将结构化引用渲染为可点击的外部链接。
 */
const CitationList: React.FC<CitationListProps> = ({ citations }) => {
  if (!citations || citations.length === 0) return null;

  return (
    <div style={{ marginTop: 8 }}>
      <div style={{ fontSize: 12, color: '#8c8c8c', marginBottom: 4 }}>
        <LinkOutlined /> 搜索来源引用
      </div>
      <Space direction="vertical" size={2} style={{ width: '100%' }}>
        {citations.map((c, idx) => (
          <div key={idx} style={{ fontSize: 12 }}>
            <Link
              href={c.url}
              target="_blank"
              rel="noopener noreferrer"
              style={{ fontSize: 12 }}
            >
              {c.title}
            </Link>
            {c.siteName && (
              <span style={{ color: '#999', marginLeft: 4 }}>({c.siteName})</span>
            )}
          </div>
        ))}
      </Space>
    </div>
  );
};

export default React.memo(CitationList);

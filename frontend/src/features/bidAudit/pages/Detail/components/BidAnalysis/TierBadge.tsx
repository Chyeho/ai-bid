import React from 'react';
import { Tag } from 'antd';
import { ArrowRightOutlined } from '@ant-design/icons';
import type { RiskTier } from '@/types/audit';
import { TIER_MAP, TIER_COLORS } from '@/types/audit';

interface TierBadgeProps {
  initialTier?: RiskTier;
  finalTier?: RiskTier;
  tierEscalated?: boolean;
}

/**
 * 风险分级标签 — 对齐 Rust RiskTier (L1/L2/L3)。
 * 若发生过动态升级 (tierEscalated=true) 则显示升级箭头。
 */
const TierBadge: React.FC<TierBadgeProps> = ({ initialTier, finalTier, tierEscalated }) => {
  const tier = finalTier || initialTier;
  if (!tier) return null;

  if (tierEscalated && initialTier && finalTier && initialTier !== finalTier) {
    return (
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 2 }}>
        <Tag color={TIER_COLORS[initialTier]} style={{ margin: 0 }}>
          {TIER_MAP[initialTier]}
        </Tag>
        <ArrowRightOutlined style={{ fontSize: 12, color: '#999' }} />
        <Tag color={TIER_COLORS[finalTier]} style={{ margin: 0 }}>
          {TIER_MAP[finalTier]}
        </Tag>
      </span>
    );
  }

  return (
    <Tag color={TIER_COLORS[tier]} style={{ margin: 0 }}>
      {TIER_MAP[tier]}
    </Tag>
  );
};

export default React.memo(TierBadge);

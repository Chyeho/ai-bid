import React from 'react';
import { Typography, Spin } from 'antd';
import {
  LoadingOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
} from '@ant-design/icons';

const { Text } = Typography;

interface PipelineProgressProps {
  currentStage: string;
  isComplete: boolean;
}

/** 将 backend stage 映射为用户可读的简短描述 */
const stageLabel = (stage: string, isComplete: boolean): string => {
  if (isComplete) return '审核完成';
  if (!stage) return '等待开始';
  const upper = stage.toUpperCase();
  if (upper.includes('UPLOAD')) return '正在解析文档…';
  if (upper.includes('EXTRACT') || upper.includes('DOC')) return '正在提取文档内容…';
  if (upper.includes('REVIEW') || upper.includes('审')) return '正在智能分析中，请耐心等待…';
  if (upper.includes('SUMM') || upper.includes('汇总')) return '正在汇总结果…';
  if (upper.includes('PEND') || upper.includes('创建')) return '正在准备…';
  return stage;
};

/**
 * 审核进度指示 — 后端审核是同步阻塞调用，无细粒度进度，
 * 因此使用不间断旋转动画表示"进行中"。
 */
const PipelineProgress: React.FC<PipelineProgressProps> = ({ currentStage, isComplete }) => {
  const label = stageLabel(currentStage, isComplete);

  if (isComplete) {
    return (
      <div style={{ padding: '6px 0', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 6 }}>
        <CheckCircleOutlined style={{ color: '#52c41a', fontSize: 14 }} />
        <Text style={{ fontSize: 13 }}>{label}</Text>
      </div>
    );
  }

  // 审核失败的情况（由使用方判断是否需要特殊样式，这里保守处理）
  if (currentStage && currentStage.toUpperCase().includes('FAIL')) {
    return (
      <div style={{ padding: '6px 0', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 6 }}>
        <CloseCircleOutlined style={{ color: '#f5222d', fontSize: 14 }} />
        <Text type="danger" style={{ fontSize: 13 }}>{label}</Text>
      </div>
    );
  }

  return (
    <div style={{ padding: '6px 0', marginBottom: 4, display: 'flex', alignItems: 'center', gap: 8 }}>
      <Spin indicator={<LoadingOutlined style={{ fontSize: 14 }} spin />} />
      <Text style={{ fontSize: 13 }}>{label}</Text>
    </div>
  );
};

export default React.memo(PipelineProgress);

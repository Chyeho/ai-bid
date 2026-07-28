import { describe, it, expect } from 'vitest';
import {
  buildClauseMap,
  buildSectionTree,
  clauseStatus,
  sectionStatus,
} from './buildSectionTree';
import type {
  FindingAddedEvent,
  TraceEvent,
  AgentProgress,
  AuditIssue,
  ClauseState,
  SectionTreeNode,
  Severity,
} from '@/types/audit';

// ─── Helpers ────────────────────────────────────────────────────────────────

function mockFinding(overrides: Partial<FindingAddedEvent> = {}): FindingAddedEvent {
  return {
    risk_id: 'risk-001',
    severity: 'medium',
    risk_type: '资质风险',
    agent: 'factcheck',
    confidence: 0.85,
    clause_ids: ['clause-1'],
    source_quote: '投标人应具有...',
    legal_basis: [],
    reason: '资质不符合要求',
    suggestion: '补充资质证明',
    lifecycle: 'verified',
    ...overrides,
  };
}

function mockIssue(overrides: Partial<AuditIssue> = {}): AuditIssue {
  return {
    issueNo: 'ISSUE-001',
    severity: 'high',
    category: '报价风险',
    description: '报价超出预算',
    suggestion: '重新报价',
    ...overrides,
  };
}

function mockTrace(overrides: Partial<TraceEvent> = {}): TraceEvent {
  return {
    event_type: 'agent_thought',
    agent_name: 'factcheck',
    turn: 1,
    clause_id: 'clause-1',
    summary: '开始审查条款1',
    ...overrides,
  };
}

function mockProgress(overrides: Partial<AgentProgress> = {}): AgentProgress {
  return {
    agent_id: 'factcheck',
    agent_label: '事实核验',
    clauses_done: 5,
    clauses_total: 10,
    raw_findings: 2,
    status: 'completed',
    ...overrides,
  };
}

/**
 * severityOrder() is not exported, so it is tested indirectly via
 * buildSectionTree / clauseStatus / sectionStatus which all exercise the
 * severity comparison logic internally.
 *
 * The order is: info=1, low=2, medium=3, high=4
 * (higher number == more severe)
 */

// ─── buildClauseMap ─────────────────────────────────────────────────────────

describe('buildClauseMap()', () => {
  it('returns an empty map when all inputs are empty', () => {
    const map = buildClauseMap([], [], [], new Map());
    expect(map.size).toBe(0);
  });

  it('creates a ClauseState entry for each clause in a live finding', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
    ];
    const map = buildClauseMap(findings, [], [], new Map());

    expect(map.size).toBe(1);
    expect(map.has('clause-1')).toBe(true);
    const c = map.get('clause-1')!;
    expect(c.clauseId).toBe('clause-1');
    expect(c.status).toBe('pending');
    expect(c.risks).toHaveLength(1);
    expect(c.risks[0].risk_id).toBe('risk-001');
  });

  it('merges multiple findings into the same clause and deduplicates by risk_id', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
      mockFinding({ risk_id: 'risk-002', clause_ids: ['clause-1'], severity: 'high' }),
      // duplicate risk_id — should be ignored
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
    ];
    const map = buildClauseMap(findings, [], [], new Map());

    expect(map.size).toBe(1);
    expect(map.get('clause-1')!.risks).toHaveLength(2);
  });

  it('handles a finding that references multiple clause_ids', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1', 'clause-2'] }),
    ];
    const map = buildClauseMap(findings, [], [], new Map());

    expect(map.size).toBe(2);
    expect(map.get('clause-1')!.risks).toHaveLength(1);
    expect(map.get('clause-2')!.risks).toHaveLength(1);
  });

  it('takes the longer section_path when merging the same clause from different sources', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'], section_path: ['第一章'] }),
      mockFinding({ risk_id: 'risk-002', clause_ids: ['clause-1'], section_path: ['第一章', '第一节'] }),
    ];
    const map = buildClauseMap(findings, [], [], new Map());

    const c = map.get('clause-1')!;
    expect(c.sectionPath).toEqual(['第一章', '第一节']);
  });

  it('maps issues (with clauseIds) into clause entries', () => {
    const issues: AuditIssue[] = [
      mockIssue({
        issueNo: 'ISS-001',
        severity: 'high',
        category: '报价风险',
        clauseIds: ['clause-1'],
        location: { pageNumber: 3, sectionName: '第二章 > 第一节', context: '' },
      }),
    ];
    const map = buildClauseMap([], issues, [], new Map());

    expect(map.size).toBe(1);
    const c = map.get('clause-1')!;
    expect(c.sectionPath).toEqual(['第二章', '第一节']);
    expect(c.risks).toHaveLength(1);
    expect(c.risks[0].risk_id).toBe('ISS-001');
    expect(c.risks[0].severity).toBe('high');
  });

  it('marks a clause as "reviewing" when a trace event references it', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
    ];
    const traces: TraceEvent[] = [
      mockTrace({ clause_id: 'clause-1' }),
    ];
    const map = buildClauseMap(findings, [], traces, new Map());

    const c = map.get('clause-1')!;
    expect(c.status).toBe('reviewing');
  });

  it('does not downgrade "reviewing" status from a trace event', () => {
    // This is an implicit test: trace events only upgrade pending→reviewing
    // so if status is already reviewing, it stays reviewing
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
    ];
    const traces: TraceEvent[] = [
      mockTrace({ clause_id: 'clause-1' }),
      mockTrace({ clause_id: 'clause-1' }), // second trace should not change anything
    ];
    const map = buildClauseMap(findings, [], traces, new Map());

    expect(map.get('clause-1')!.status).toBe('reviewing');
  });

  it('marks all clauses as "reviewed" and fills reviewedBy when any agent completes', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
      mockFinding({ risk_id: 'risk-002', clause_ids: ['clause-2'] }),
    ];
    const progresses = new Map<string, AgentProgress>([
      ['factcheck', mockProgress({ agent_label: '事实核验', status: 'completed' })],
    ]);
    const map = buildClauseMap(findings, [], [], progresses);

    for (const [, clause] of map) {
      expect(clause.status).toBe('reviewed');
      expect(clause.reviewedBy).toContain('事实核验');
    }
  });

  it('accumulates multiple reviewedBy entries from different completed agents', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({ risk_id: 'risk-001', clause_ids: ['clause-1'] }),
    ];
    const progresses = new Map<string, AgentProgress>([
      ['factcheck', mockProgress({ agent_label: '事实核验', status: 'completed' })],
      ['procedure', mockProgress({ agent_label: '流程合规', status: 'completed' })],
    ]);
    const map = buildClauseMap(findings, [], [], progresses);

    const c = map.get('clause-1')!;
    expect(c.reviewedBy).toContain('事实核验');
    expect(c.reviewedBy).toContain('流程合规');
  });
});

// ─── buildSectionTree ───────────────────────────────────────────────────────

describe('buildSectionTree()', () => {
  it('returns an empty array when the clause map is empty', () => {
    const tree = buildSectionTree(new Map());
    expect(tree).toEqual([]);
  });

  it('creates a single leaf node for a clause with a shallow section path', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['第一章'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-001', severity: 'high' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree).toHaveLength(1);
    expect(tree[0].key).toBe('第一章');
    expect(tree[0].clauseIds).toEqual(['clause-1']);
    expect(tree[0].riskCount).toBe(1);
    expect(tree[0].maxSeverity).toBe('high');
    expect(tree[0].children).toEqual([]);
  });

  it('groups multiple clauses under the same section path into one node', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['第一章'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-001', severity: 'low' })],
      }],
      ['clause-2', {
        clauseId: 'clause-2',
        sectionPath: ['第一章'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-002', severity: 'medium' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree).toHaveLength(1);
    const node = tree[0];
    expect(node.clauseIds).toEqual(['clause-1', 'clause-2']);
    // 1 low + 1 medium = 2 risks; maxSeverity should be the highest (medium)
    expect(node.riskCount).toBe(2);
    expect(node.maxSeverity).toBe('medium');
  });

  it('builds nested children from multi-level section paths', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['第一章', '第一节'],
        status: 'pending',
        reviewedBy: [],
        // 2 risks -> sorts before 第二节 (1 risk)
        risks: [
          mockFinding({ risk_id: 'risk-001', severity: 'high' }),
          mockFinding({ risk_id: 'risk-004', severity: 'medium' }),
        ],
      }],
      ['clause-2', {
        clauseId: 'clause-2',
        sectionPath: ['第一章', '第二节'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-002', severity: 'low' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree).toHaveLength(1);
    const ch1 = tree[0];
    expect(ch1.key).toBe('第一章');
    expect(ch1.children).toHaveLength(2);

    // Sorted by riskCount desc: 第一节 (2 risks) before 第二节 (1 risk)
    const sec1 = ch1.children[0];
    const sec2 = ch1.children[1];
    expect(sec1.title).toBe('第一节');
    expect(sec1.riskCount).toBe(2);
    expect(sec1.maxSeverity).toBe('high');
    expect(sec1.clauseIds).toEqual(['clause-1']);

    expect(sec2.title).toBe('第二节');
    expect(sec2.riskCount).toBe(1);
    expect(sec2.maxSeverity).toBe('low');
    expect(sec2.clauseIds).toEqual(['clause-2']);

    // Parent node propagates aggregate stats from children
    expect(ch1.riskCount).toBe(3);
    expect(ch1.maxSeverity).toBe('high');
    expect(ch1.clauseIds).toEqual([]);
  });

  it('propagates riskCount and maxSeverity from children to parent', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['第一章', '第一节'],
        status: 'pending',
        reviewedBy: [],
        risks: [
          mockFinding({ risk_id: 'risk-001', severity: 'high' }),
          mockFinding({ risk_id: 'risk-002', severity: 'medium' }),
        ],
      }],
      ['clause-2', {
        clauseId: 'clause-2',
        sectionPath: ['第一章', '第二节'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-003', severity: 'low' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree).toHaveLength(1);
    const parent = tree[0];
    // Total in children: 2 (第一节) + 1 (第二节) = 3
    expect(parent.riskCount).toBe(3);
    expect(parent.maxSeverity).toBe('high');
    // Parent should not have its own clauseIds (it's a branch, not a leaf)
    expect(parent.clauseIds).toEqual([]);
  });

  it('filters out "info" severity risks from riskCount', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['第一章'],
        status: 'pending',
        reviewedBy: [],
        risks: [
          mockFinding({ risk_id: 'risk-info', severity: 'info' }),
          mockFinding({ risk_id: 'risk-high', severity: 'high' }),
        ],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree[0].riskCount).toBe(1);
    expect(tree[0].maxSeverity).toBe('high');
  });

  it('falls back to "其他条款" when sectionPath is empty', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-orphan', {
        clauseId: 'clause-orphan',
        sectionPath: [],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'risk-001', severity: 'medium' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    expect(tree).toHaveLength(1);
    expect(tree[0].key).toBe('其他条款');
  });

  it('sorts nodes so higher riskCount sections appear first', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-a', {
        clauseId: 'clause-a',
        sectionPath: ['第二节'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'r1', severity: 'low' })],
      }],
      ['clause-b', {
        clauseId: 'clause-b',
        sectionPath: ['第一节'],
        status: 'pending',
        reviewedBy: [],
        risks: [
          mockFinding({ risk_id: 'r2', severity: 'high' }),
          mockFinding({ risk_id: 'r3', severity: 'medium' }),
        ],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    // 第一节 (2 risks) should sort before 第二节 (1 risk)
    expect(tree).toHaveLength(2);
    expect(tree[0].title).toBe('第一节');
    expect(tree[1].title).toBe('第二节');
  });

  it('sorts nodes with same riskCount by Chinese locale title', () => {
    const clauseMap = new Map<string, ClauseState>([
      ['clause-2', {
        clauseId: 'clause-2',
        sectionPath: ['乙章'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'r1', severity: 'low' })],
      }],
      ['clause-1', {
        clauseId: 'clause-1',
        sectionPath: ['甲章'],
        status: 'pending',
        reviewedBy: [],
        risks: [mockFinding({ risk_id: 'r2', severity: 'low' })],
      }],
    ]);
    const tree = buildSectionTree(clauseMap);

    // Same riskCount=1, sorted by zh locale: 甲 before 乙
    expect(tree).toHaveLength(2);
    expect(tree[0].title).toBe('甲章');
    expect(tree[1].title).toBe('乙章');
  });
});

// ─── clauseStatus ───────────────────────────────────────────────────────────

describe('clauseStatus()', () => {
  it('returns "pending" for a clause with status "pending" and no reviewedBy', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'pending',
      reviewedBy: [],
      risks: [],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('pending');
    expect(result.color).toBe('#d9d9d9');
    expect(result.label).toBe('待审');
  });

  it('returns "reviewing" for a clause with status "reviewing"', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewing',
      reviewedBy: [],
      risks: [],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('reviewing');
    expect(result.color).toBe('#1890ff');
    expect(result.label).toBe('审查中');
  });

  it('returns "clean" for a reviewed clause with no active risks (only info risks)', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: ['事实核验'],
      risks: [mockFinding({ risk_id: 'r1', severity: 'info' })],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('clean');
    expect(result.color).toBe('#52c41a');
    expect(result.label).toContain('1 Agent已审');
  });

  it('returns "clean" for a reviewed clause with zero risks', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: [],
      risks: [],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('clean');
    expect(result.label).toBe('已审');
  });

  it('returns "risk_high" when a high severity risk exists', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: ['事实核验'],
      risks: [mockFinding({ risk_id: 'r1', severity: 'high' })],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('risk_high');
    expect(result.color).toBe('#f5222d');
    expect(result.label).toBe('1处风险');
  });

  it('returns "risk_medium" when only medium severity risks exist', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: ['事实核验'],
      risks: [mockFinding({ risk_id: 'r1', severity: 'medium' })],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('risk_medium');
    expect(result.color).toBe('#fa8c16');
    expect(result.label).toBe('1处风险');
  });

  it('returns "risk_low" when only low severity risks exist', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: ['事实核验'],
      risks: [mockFinding({ risk_id: 'r1', severity: 'low' })],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('risk_low');
    expect(result.color).toBe('#1890ff');
    expect(result.label).toBe('1处风险');
  });

  it('prefers "high" over lower severities in the same clause', () => {
    const clause: ClauseState = {
      clauseId: 'c1',
      sectionPath: [],
      status: 'reviewed',
      reviewedBy: [],
      risks: [
        mockFinding({ risk_id: 'r1', severity: 'low' }),
        mockFinding({ risk_id: 'r2', severity: 'high' }),
        mockFinding({ risk_id: 'r3', severity: 'medium' }),
      ],
    };
    const result = clauseStatus(clause);
    expect(result.status).toBe('risk_high');
    expect(result.label).toBe('3处风险');
  });
});

// ─── sectionStatus ──────────────────────────────────────────────────────────

describe('sectionStatus()', () => {
  it('returns "无风险" when maxSeverity is null', () => {
    const node: SectionTreeNode = {
      key: '第一章',
      title: '第一章',
      path: ['第一章'],
      clauseIds: [],
      riskCount: 0,
      maxSeverity: null,
      children: [],
    };
    const result = sectionStatus(node);
    expect(result.color).toBe('#52c41a');
    expect(result.label).toBe('无风险');
  });

  it('returns risk count label with red color for high severity', () => {
    const node: SectionTreeNode = {
      key: '第一章',
      title: '第一章',
      path: ['第一章'],
      clauseIds: ['c1'],
      riskCount: 3,
      maxSeverity: 'high',
      children: [],
    };
    const result = sectionStatus(node);
    expect(result.color).toBe('#f5222d');
    expect(result.label).toBe('3处风险');
  });

  it('returns risk count label with orange color for medium severity', () => {
    const node: SectionTreeNode = {
      key: '第一章',
      title: '第一章',
      path: ['第一章'],
      clauseIds: ['c1'],
      riskCount: 2,
      maxSeverity: 'medium',
      children: [],
    };
    const result = sectionStatus(node);
    expect(result.color).toBe('#fa8c16');
    expect(result.label).toBe('2处风险');
  });

  it('returns risk count label with blue color for low severity', () => {
    const node: SectionTreeNode = {
      key: '第一章',
      title: '第一章',
      path: ['第一章'],
      clauseIds: ['c1'],
      riskCount: 1,
      maxSeverity: 'low',
      children: [],
    };
    const result = sectionStatus(node);
    expect(result.color).toBe('#1890ff');
    expect(result.label).toBe('1处风险');
  });

  it('returns gray color for unrecognized severity', () => {
    const node: SectionTreeNode = {
      key: '未知',
      title: '未知',
      path: ['未知'],
      clauseIds: ['c1'],
      riskCount: 1,
      // Cast to Severity to simulate a value that falls through the switch
      maxSeverity: 'unknown' as Severity,
      children: [],
    };
    const result = sectionStatus(node);
    expect(result.color).toBe('#8c8c8c');
    expect(result.label).toBe('信息');
  });
});

// ─── Integration: buildClauseMap → buildSectionTree ─────────────────────────

describe('buildClauseMap + buildSectionTree integration', () => {
  it('produces a complete tree from raw findings and agents', () => {
    const findings: FindingAddedEvent[] = [
      mockFinding({
        risk_id: 'risk-001',
        severity: 'high',
        clause_ids: ['clause-1'],
        section_path: ['第一章', '第一节'],
        agent: 'factcheck',
      }),
      mockFinding({
        risk_id: 'risk-002',
        severity: 'medium',
        clause_ids: ['clause-2'],
        section_path: ['第一章', '第二节'],
        agent: 'semanticrisk',
      }),
      mockFinding({
        risk_id: 'risk-003',
        severity: 'low',
        clause_ids: ['clause-3'],
        section_path: ['第二章'],
        agent: 'procedure',
      }),
    ];
    const traces: TraceEvent[] = [
      mockTrace({ clause_id: 'clause-1' }),
    ];
    const progresses = new Map<string, AgentProgress>([
      ['factcheck', mockProgress({ agent_label: '事实核验', status: 'completed' })],
      ['semanticrisk', mockProgress({ agent_label: '风险识别', status: 'completed' })],
    ]);

    const clauseMap = buildClauseMap(findings, [], traces, progresses);
    const tree = buildSectionTree(clauseMap);

    // clause-1 should be reviewing (from trace) then reviewed (from progress)
    expect(clauseMap.get('clause-1')!.status).toBe('reviewed');
    expect(clauseMap.get('clause-2')!.status).toBe('reviewed');

    // Tree structure: 第一章 (children: 第一节, 第二节) + 第二章
    expect(tree).toHaveLength(2);

    // 第一章 (2 risks) sorts before 第二章 (1 risk)
    expect(tree[0].title).toBe('第一章');
    expect(tree[0].children).toHaveLength(2);
    // Children have different risk counts -> deterministic sort by riskCount desc
    // Give 第一节 2 risks (add a second finding for clause-1)
    findings.push(mockFinding({
      risk_id: 'risk-004',
      severity: 'low',
      clause_ids: ['clause-1'],
      section_path: ['第一章', '第一节'],
      agent: 'factcheck',
    }));
    const clauseMap2 = buildClauseMap(findings, [], traces, progresses);
    const tree2 = buildSectionTree(clauseMap2);

    expect(tree2[0].children[0].title).toBe('第一节');
    expect(tree2[0].children[0].riskCount).toBe(2);
    expect(tree2[0].children[0].maxSeverity).toBe('high');
    expect(tree2[0].children[1].title).toBe('第二节');
    expect(tree2[0].children[1].riskCount).toBe(1);
    expect(tree2[0].children[1].maxSeverity).toBe('medium');

    // 第二章
    expect(tree2[1].title).toBe('第二章');
    expect(tree2[1].riskCount).toBe(1);
    expect(tree2[1].maxSeverity).toBe('low');
  });
});

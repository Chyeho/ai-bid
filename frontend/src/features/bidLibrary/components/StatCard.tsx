import { useStyles } from '../style';

interface StatCardProps {
   categoryCounts: Record<string, number>; // 各分类文件数量
}

// --- 统计卡片组件 ---
export function StatCard({ categoryCounts }: StatCardProps) {
   const { styles } = useStyles();

   return (
      <div className={`${styles.statsCard}`}>
         <div className={styles.statsRow}>
            {/* 左侧：总文件数 + 两个分类 */}
            <div className={styles.leftColumn}>
               <div className={styles.totalStat}>
                  <span className={styles.totalLabel}>总文件数</span>
                  <span className={styles.totalNumber}>
                     {categoryCounts.all}
                  </span>
               </div>
               <div className={styles.otherStats}>
                  <div className={styles.statItem}>
                     <span className={styles.statLabel}>价格标准</span>
                     <span className={styles.statValue}>
                        {categoryCounts.price}
                     </span>
                  </div>
                  <div className={styles.statItem}>
                     <span className={styles.statLabel}>合同模板</span>
                     <span className={styles.statValue}>
                        {categoryCounts.contract}
                     </span>
                  </div>
               </div>
            </div>
            {/* 右侧：四个分类 */}
            <div className={styles.rightColumn}>
               <div className={styles.statItem}>
                  <span className={styles.statLabel}>制度文件</span>
                  <span className={styles.statValue}>
                     {categoryCounts.regulation}
                  </span>
               </div>
               <div className={styles.statItem}>
                  <span className={styles.statLabel}>供应商名录</span>
                  <span className={styles.statValue}>
                     {categoryCounts.supplier}
                  </span>
               </div>
               <div className={styles.statItem}>
                  <span className={styles.statLabel}>案例库</span>
                  <span className={styles.statValue}>
                     {categoryCounts.case}
                  </span>
               </div>
               <div className={styles.statItem}>
                  <span className={styles.statLabel}>其他</span>
                  <span className={styles.statValue}>
                     {categoryCounts.other}
                  </span>
               </div>
            </div>
         </div>
      </div>
   );
}

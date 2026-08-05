import React from 'react';
import { Input, Checkbox, Typography } from 'antd';
import { useStyles } from '../style';
import { REPORT_SECTIONS } from '../utils';

const { Text } = Typography;

interface ReportSettingsProps {
   selectedSections: string[];
   onSelectionChange: (checkedValues: string[]) => void;
   fileName: string;
   onFileNameChange: (name: string) => void;
}

export const ReportSettings: React.FC<ReportSettingsProps> = ({
   selectedSections,
   onSelectionChange,
   fileName,
   onFileNameChange,
}) => {
   const { styles } = useStyles();

   // 处理 antd CheckboxGroup 的类型兼容
   const handleCheckboxChange = (checkedValues: unknown) => {
      onSelectionChange(checkedValues as string[]);
   };

   return (
      <aside className={styles.settingsArea}>
         {/* 1. 导出格式 (只读) */}
         <div className={styles.settingSection}>
            <div className={styles.settingLabel}>导出格式</div>
            <Input value='Word (.docx)' disabled />
         </div>

         {/* 2. 报告内容配置 (多选联动) */}
         <div className={styles.settingSection}>
            <div className={styles.settingLabel}>报告内容配置</div>
            <Checkbox.Group
               className={styles.checkboxGroup}
               options={REPORT_SECTIONS.map((sec) => ({
                  label: sec,
                  value: sec,
               }))}
               value={selectedSections}
               onChange={handleCheckboxChange}
            />
            <Text
               type='secondary'
               style={{ fontSize: '12px', marginTop: '4px' }}
            >
               已选 {selectedSections.length}/{REPORT_SECTIONS.length} 个模块
            </Text>
         </div>

         {/* 3. 文件名配置 */}
         <div className={styles.settingSection}>
            <div className={styles.settingLabel}>文件名配置</div>
            <Input
               value={fileName}
               onChange={(e) => onFileNameChange(e.target.value)}
               placeholder='请输入导出的文件名'
               suffix='.docx'
               allowClear
            />
         </div>
      </aside>
   );
};

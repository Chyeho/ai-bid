import { useState, useEffect } from 'react';
import { Button, Input } from 'antd';
import { SearchOutlined, UploadOutlined } from '@ant-design/icons';
import { useStyles } from '../style';
import { useDebounce } from '@/hooks/useDebounce';

interface SearchBarProps {
   searchKeyword: string;
   onSearchChange: (value: string) => void;
   onUploadClick: () => void;
}

export function SearchBar({
   searchKeyword,
   onSearchChange,
   onUploadClick,
}: SearchBarProps) {
   const { styles } = useStyles();

   // 1. 本地状态管理输入值，保证输入框不卡顿
   const [localValue, setLocalValue] = useState(searchKeyword);

   // 2. 当父组件传入的 searchKeyword 变化时（比如 URL 变更），同步更新本地状态
   useEffect(() => {
      setLocalValue(searchKeyword);
   }, [searchKeyword]);

   // 3. 定义防抖触发动作
   const { run: debouncedSearch } = useDebounce((val: string) => {
      onSearchChange(val);
   }, 500);

   const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const val = e.target.value;
      setLocalValue(val); // 实时更新输入框
      debouncedSearch(val); // 触发防抖查询
   };

   return (
      <div
         className={`${styles.headerRow}`}
      >
         <Input
            placeholder='搜索文件名或内容...'
            prefix={<SearchOutlined />}
            className={styles.searchInput}
            value={localValue}
            onChange={handleChange}
            allowClear
         />
         <Button
            type='primary'
            icon={<UploadOutlined />}
            className={styles.uploadBtn}
            onClick={onUploadClick}
         >
            上传文件
         </Button>
      </div>
   );
}

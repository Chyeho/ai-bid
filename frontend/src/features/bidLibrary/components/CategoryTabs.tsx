import React from 'react';
import {
   InboxOutlined,
   FileTextOutlined,
   TagsOutlined,
   UnorderedListOutlined,
   FileDoneOutlined,
   FolderOutlined,
   MoreOutlined,
} from '@ant-design/icons';
import { useStyles } from '../style';
import { CategoryMap } from '../types';

const categories = [
   'all',
   'regulation',
   'price',
   'supplier',
   'contract',
   'case',
   'other',
] as const;

const CategoryLabelMap: Record<string, string> = {
   all: '全部',
   ...CategoryMap,
};

const CategoryIconMap: Record<string, React.ReactNode> = {
   all: <InboxOutlined />,
   regulation: <FileTextOutlined />,
   price: <TagsOutlined />,
   supplier: <UnorderedListOutlined />,
   contract: <FileDoneOutlined />,
   case: <FolderOutlined />,
   other: <MoreOutlined />,
};

// --- 分类标签组件 Props ---
interface CategoryTabsProps {
   selectedCategory: string;
   onCategoryChange: (category: string) => void;
}

// --- 分类标签组件 ---
export function CategoryTabs({
   selectedCategory,
   onCategoryChange,
}: CategoryTabsProps) {
   const { styles } = useStyles();

   return (
      <div className={styles.categoryTabs}>
         {categories.map((cat) => (
            <div
               key={cat}
               className={`${styles.categoryTab} ${
                  selectedCategory === cat ? 'active' : ''
               }`}
               onClick={() => onCategoryChange(cat)}
            >
               {CategoryIconMap[cat]}
               <span>{CategoryLabelMap[cat]}</span>
            </div>
         ))}
      </div>
   );
}

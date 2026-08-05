import { useState } from 'react';

import { useIsMobile } from '@/hooks/useMediaQuery';

import { DatePicker, Space } from 'antd';
import { CalendarOutlined } from '@ant-design/icons';
import type { Dayjs } from 'dayjs';

const { RangePicker } = DatePicker;

type DateRangeValue = [Dayjs | null, Dayjs | null] | null;

type RangePickerProps = {
   value?: DateRangeValue;
   onChange?: (value: DateRangeValue) => void;
   placeholder?: [string, string];
   style?: React.CSSProperties;
};

export const ResponsiveRangePicker: React.FC<RangePickerProps> = ({
   value,
   onChange,
   ...props
}) => {
   const isMobile = useIsMobile();
   const [startDate, setStartDate] = useState(value?.[0]);
   const [endDate, setEndDate] = useState(value?.[1]);

   if (isMobile) {
      return (
         <Space direction='vertical' style={{ width: '100%' }}>
            <DatePicker
               placeholder='开始日期'
               prefix={<CalendarOutlined />}
               value={startDate}
               onChange={(date) => {
                  setStartDate(date);
                  onChange?.([date, endDate ?? null]);
               }}
               style={{ width: '100%', height: 35 }}
            />
            <DatePicker
               placeholder='结束日期'
               prefix={<CalendarOutlined />}
               value={endDate}
               onChange={(date) => {
                  setEndDate(date);
                  onChange?.([startDate ?? null, date]);
               }}
               style={{ width: '100%', height: 35 }}
               disabledDate={(current) => {
                  if (!startDate) return false;
                  return current.isBefore(startDate);
               }}
            />
         </Space>
      );
   }

   return (
      <RangePicker
         value={value}
         onChange={(dates) => onChange?.(dates)}
         placeholder={['开始时间', '结束时间']}
         prefix={<CalendarOutlined />}
         style={{ width: 280, height: 35 }}
         {...props}
      />
   );
};

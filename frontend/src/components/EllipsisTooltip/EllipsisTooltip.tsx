import { Tooltip } from 'antd';

interface EllipsisTooltipProps {
   text: string;
}

export const EllipsisTooltip: React.FC<EllipsisTooltipProps> = ({ text }) => {
   return (
      <Tooltip
         placement='topLeft'
         title={
            <div
               style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 4,
               }}
            >
               {text}
            </div>
         }
         styles={{ root: { maxWidth: 300 } }}
      >
         <span
            style={{
               display: 'inline-block',
               maxWidth: '100%',
               overflow: 'hidden',
               textOverflow: 'ellipsis',
               whiteSpace: 'nowrap',
            }}
         >
            {text ?? '-'}
         </span>
      </Tooltip>
   );
};

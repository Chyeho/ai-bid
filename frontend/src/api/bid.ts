import request from '@/api/request';
import { queryOptions } from '@tanstack/react-query';
import type { BaseResponse } from '@/api/types';

export interface BidDetail {
   id: number;
   bidName: string;
}

export const getBidDetail = async (id: number | string): Promise<BidDetail> => {
   const res = await request.get<unknown, BaseResponse<BidDetail>>(
      `/api/bid-documents/${id}`
   );
   return res.data;
};

export const bidOptions = {
   detail: (id: number | string) =>
      queryOptions({
         queryKey: ['bidDetail', id],
         queryFn: () => getBidDetail(id),
         enabled: !!id,
         staleTime: 10 * 60 * 1000,
      }),
};

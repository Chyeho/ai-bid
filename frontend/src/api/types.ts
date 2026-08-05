export interface PageParams {
   page: number;
   size: number;
}

export interface PageResponse<T> {
   records: T[];
   total: number;
}

export interface BaseResponse<T> {
   code: number;
   msg: string;
   data: T;
   timestamp: number;
}

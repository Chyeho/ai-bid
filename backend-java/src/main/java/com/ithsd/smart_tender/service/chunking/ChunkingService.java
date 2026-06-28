package com.ithsd.smart_tender.service.chunking;

import com.ithsd.smart_tender.service.extract.model.ParsedBlock;

import java.util.List;

public interface ChunkingService {
    List<ChunkSlice> chunk(List<ParsedBlock> blocks);
}

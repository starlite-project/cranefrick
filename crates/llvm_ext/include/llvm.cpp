#include "llvm-c/Core.h"
#include "llvm/IR/Metadata.h"
#include <vector>

using namespace llvm;

#ifdef __cplusplus
extern "C"
{
#endif

    LLVMMetadataRef LLVMCreateSelfReferentialNodeInContext(LLVMContextRef C, LLVMMetadataRef *Nodes, unsigned Count) {
        auto temp = MDNode::getTemporary(*unwrap(C), {});
        SmallVector<Metadata*, 1> ops = {temp.get()};
        for (unsigned i = 0; i < Count; i++)
            ops.push_back(reinterpret_cast<Metadata*>(Nodes[i]));
        MDNode* ret = MDNode::get(*unwrap(C), ops);
        temp->replaceAllUsesWith(ret);
        return wrap(ret);
    }

    LLVMMetadataRef LLVMCreateDistinctNodeInContext(LLVMContextRef C, LLVMMetadataRef *Nodes, unsigned Count)
    {
        SmallVector<Metadata *, 4> vec;
        for (unsigned i = 0; i < Count; i++)
            vec.push_back(reinterpret_cast<Metadata *>(Nodes[i]));

        MDNode *node = MDNode::getDistinct(*unwrap(C), vec);

        return wrap(node);
    }

    LLVMMetadataRef LLVMCreateSelfReferentialDistinctNodeInContext(LLVMContextRef C, LLVMMetadataRef *Nodes, unsigned Count)
    {
        auto temp = MDNode::getTemporary(*unwrap(C), {});
        SmallVector<Metadata *, 1> ops = {temp.get()};
        for (unsigned i = 0; i < Count; i++)
            ops.push_back(reinterpret_cast<Metadata*>(Nodes[i]));
        MDNode* ret = MDNode::getDistinct(*unwrap(C), ops);
        temp->replaceAllUsesWith(ret);
        return wrap(ret);
    }

#ifdef __cplusplus
}
#endif

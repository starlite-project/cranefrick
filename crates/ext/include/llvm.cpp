#include "llvm-c/Core.h"
#include "llvm-c/Types.h"
#include "llvm/IR/Attributes.h"
#include "llvm/IR/BasicBlock.h"

using namespace llvm;

extern "C"
{
    LLVMMetadataRef LLVMMDDistinctNodeInContext2(LLVMContextRef C, LLVMMetadataRef *MDs, size_t Count)
    {
        return wrap(MDNode::getDistinct(*unwrap(C), ArrayRef<Metadata *>(unwrap(MDs), Count)));
    }
}

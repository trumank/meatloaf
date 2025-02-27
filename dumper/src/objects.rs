use crate::{
    containers::{FName, FString, TArray, TTuple},
    mem::{CtxPtr, ExternalPtr, Mem, StructsTrait},
};
use anyhow::Result;
use ue_reflection::{EClassCastFlags, EClassFlags, EFunctionFlags, EPropertyFlags, EStructFlags};

#[derive(Clone, Copy)]
pub struct UObject;
impl<C: Clone + StructsTrait> CtxPtr<UObject, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        let offset = self.ctx().struct_member("UObjectBase", "ClassPrivate");
        self.byte_offset(offset).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        let offset = self.ctx().struct_member("UObjectBase", "NamePrivate");
        self.byte_offset(offset).cast()
    }
    pub fn outer_private(&self) -> CtxPtr<Option<ExternalPtr<UObject>>, C> {
        let offset = self.ctx().struct_member("UObjectBase", "OuterPrivate");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UField;
impl<C: Clone> CtxPtr<UField, C> {
    pub fn uobject(&self) -> CtxPtr<UObject, C> {
        self.cast()
    }
}

#[derive(Clone, Copy)]
pub struct UStruct;
impl<C: Clone + StructsTrait> CtxPtr<UStruct, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn super_struct(&self) -> CtxPtr<Option<ExternalPtr<UStruct>>, C> {
        let offset = self.ctx().struct_member("UStruct", "SuperStruct");
        self.byte_offset(offset).cast()
    }
    pub fn child_properties(&self) -> CtxPtr<Option<ExternalPtr<FField>>, C> {
        let offset = self.ctx().struct_member("UStruct", "ChildProperties");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UClass;
impl<C: Clone + StructsTrait> CtxPtr<UClass, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn class_flags(&self) -> CtxPtr<EClassFlags, C> {
        let offset = self.ctx().struct_member("UClass", "ClassFlags");
        self.byte_offset(offset).cast()
    }
    pub fn class_cast_flags(&self) -> CtxPtr<EClassCastFlags, C> {
        let offset = self.ctx().struct_member("UClass", "ClassCastFlags");
        self.byte_offset(offset).cast()
    }
    pub fn class_default_object(&self) -> CtxPtr<Option<ExternalPtr<UObject>>, C> {
        let offset = self.ctx().struct_member("UClass", "ClassDefaultObject");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UScriptStruct;
impl<C: Clone + StructsTrait> CtxPtr<UScriptStruct, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn struct_flags(&self) -> CtxPtr<EStructFlags, C> {
        let offset = self.ctx().struct_member("UScriptStruct", "StructFlags");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UFunction;
impl<C: Clone + StructsTrait> CtxPtr<UFunction, C> {
    pub fn ustruct(&self) -> CtxPtr<UStruct, C> {
        self.cast()
    }
    pub fn function_flags(&self) -> CtxPtr<EFunctionFlags, C> {
        let offset = self.ctx().struct_member("UFunction", "FunctionFlags");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct UEnum;
impl<C: Clone + StructsTrait> CtxPtr<UEnum, C> {
    pub fn ufield(&self) -> CtxPtr<UField, C> {
        self.cast()
    }
    pub fn cpp_type(&self) -> CtxPtr<FString, C> {
        let offset = self.ctx().struct_member("UEnum", "CppType");
        self.byte_offset(offset).cast()
    }
    pub fn names(&self) -> CtxPtr<TArray<TTuple<FName, i64>>, C> {
        let offset = self.ctx().struct_member("UEnum", "Names");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FField;
impl<C: Clone + StructsTrait> CtxPtr<FField, C> {
    pub fn class_private(&self) -> CtxPtr<ExternalPtr<FFieldClass>, C> {
        let offset = self.ctx().struct_member("FField", "ClassPrivate");
        self.byte_offset(offset).cast()
    }
    pub fn next(&self) -> CtxPtr<Option<ExternalPtr<FField>>, C> {
        let offset = self.ctx().struct_member("FField", "Next");
        self.byte_offset(offset).cast()
    }
    pub fn name_private(&self) -> CtxPtr<FName, C> {
        let offset = self.ctx().struct_member("FField", "NamePrivate");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FFieldClass;
impl<C: Clone + StructsTrait> CtxPtr<FFieldClass, C> {
    pub fn cast_flags(&self) -> CtxPtr<EClassCastFlags, C> {
        let offset = self.ctx().struct_member("FFieldClass", "CastFlags");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FProperty;
impl<C: Clone + StructsTrait> CtxPtr<FProperty, C> {
    pub fn ffield(&self) -> CtxPtr<FField, C> {
        self.cast()
    }
    pub fn element_size(&self) -> CtxPtr<i32, C> {
        let offset = self.ctx().struct_member("FProperty", "ElementSize");
        self.byte_offset(offset).cast()
    }
    pub fn property_flags(&self) -> CtxPtr<EPropertyFlags, C> {
        let offset = self.ctx().struct_member("FProperty", "PropertyFlags");
        self.byte_offset(offset).cast()
    }
    pub fn offset_internal(&self) -> CtxPtr<i32, C> {
        let offset = self.ctx().struct_member("FProperty", "Offset_Internal");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FBoolProperty;
impl<C: Clone + StructsTrait> CtxPtr<FBoolProperty, C> {
    pub fn field_size(&self) -> CtxPtr<u8, C> {
        let offset = self.ctx().struct_member("FBoolProperty", "FieldSize");
        self.byte_offset(offset).cast()
    }
    pub fn byte_offset_(&self) -> CtxPtr<u8, C> {
        let offset = self.ctx().struct_member("FBoolProperty", "ByteOffset");
        self.byte_offset(offset).cast()
    }
    pub fn byte_mask(&self) -> CtxPtr<u8, C> {
        let offset = self.ctx().struct_member("FBoolProperty", "ByteMask");
        self.byte_offset(offset).cast()
    }
    pub fn field_mask(&self) -> CtxPtr<u8, C> {
        let offset = self.ctx().struct_member("FBoolProperty", "FieldMask");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FObjectProperty;
impl<C: Clone + StructsTrait> CtxPtr<FObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<Option<ExternalPtr<UClass>>, C> {
        let offset = self
            .ctx()
            .struct_member("FObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FSoftObjectProperty;
impl<C: Clone + StructsTrait> CtxPtr<FSoftObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        let offset = self
            .ctx()
            .struct_member("FObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FWeakObjectProperty;
impl<C: Clone + StructsTrait> CtxPtr<FWeakObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        let offset = self
            .ctx()
            .struct_member("FObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FLazyObjectProperty;
impl<C: Clone + StructsTrait> CtxPtr<FLazyObjectProperty, C> {
    pub fn property_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        let offset = self
            .ctx()
            .struct_member("FObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FInterfaceProperty;
impl<C: Clone + StructsTrait> CtxPtr<FInterfaceProperty, C> {
    pub fn interface_class(&self) -> CtxPtr<ExternalPtr<UClass>, C> {
        let offset = self
            .ctx()
            .struct_member("FObjectPropertyBase", "PropertyClass");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FArrayProperty;
impl<C: Clone + StructsTrait> CtxPtr<FArrayProperty, C> {
    pub fn inner(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        let offset = self.ctx().struct_member("FArrayProperty", "Inner");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FStructProperty;
impl<C: Clone + StructsTrait> CtxPtr<FStructProperty, C> {
    pub fn struct_(&self) -> CtxPtr<ExternalPtr<UScriptStruct>, C> {
        let offset = self.ctx().struct_member("FStructProperty", "Struct");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FMapProperty;
impl<C: Clone + StructsTrait> CtxPtr<FMapProperty, C> {
    pub fn key_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        let offset = self.ctx().struct_member("FMapProperty", "KeyProp");
        self.byte_offset(offset).cast()
    }
    pub fn value_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        let offset = self.ctx().struct_member("FMapProperty", "ValueProp");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FSetProperty;
impl<C: Clone + StructsTrait> CtxPtr<FSetProperty, C> {
    pub fn element_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        let offset = self.ctx().struct_member("FSetProperty", "ElementProp");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FEnumProperty;
impl<C: Clone + StructsTrait> CtxPtr<FEnumProperty, C> {
    pub fn underlying_prop(&self) -> CtxPtr<ExternalPtr<FProperty>, C> {
        let offset = self.ctx().struct_member("FEnumProperty", "UnderlyingProp");
        self.byte_offset(offset).cast()
    }
    pub fn enum_(&self) -> CtxPtr<Option<ExternalPtr<UEnum>>, C> {
        let offset = self.ctx().struct_member("FEnumProperty", "Enum");
        self.byte_offset(offset).cast()
    }
}
#[derive(Clone, Copy)]
pub struct FByteProperty;
impl<C: Clone + StructsTrait> CtxPtr<FByteProperty, C> {
    pub fn enum_(&self) -> CtxPtr<Option<ExternalPtr<UEnum>>, C> {
        let offset = self.ctx().struct_member("FByteProperty", "Enum");
        self.byte_offset(offset).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectItem;
impl<C: Clone + StructsTrait> CtxPtr<FUObjectItem, C> {
    pub fn object(&self) -> CtxPtr<Option<ExternalPtr<UObject>>, C> {
        self.byte_offset(0).cast()
    }
}

#[derive(Clone, Copy)]
pub struct FChunkedFixedUObjectArray;
impl<C: Clone + StructsTrait> CtxPtr<FChunkedFixedUObjectArray, C> {
    pub fn objects(&self) -> CtxPtr<ExternalPtr<ExternalPtr<FUObjectItem>>, C> {
        self.byte_offset(0).cast()
    }
    pub fn num_elements(&self) -> CtxPtr<i32, C> {
        self.byte_offset(20).cast()
    }
}
impl<C: Mem + Clone + StructsTrait> CtxPtr<FChunkedFixedUObjectArray, C> {
    pub fn read_item_ptr(&self, item: usize) -> Result<CtxPtr<FUObjectItem, C>> {
        let max_per_chunk = 64 * 1024;
        let chunk_index = item / max_per_chunk;

        Ok(self
            .objects()
            .read()?
            .offset(chunk_index)
            .read()?
            .byte_offset(24 * (item % max_per_chunk))) // TODO dynamic struct size
    }
}

#[derive(Clone, Copy)]
pub struct FUObjectArray;
impl<C: Clone + StructsTrait> CtxPtr<FUObjectArray, C> {
    pub fn obj_object(&self) -> CtxPtr<FChunkedFixedUObjectArray, C> {
        self.byte_offset(16).cast()
    }
}

mod containers;
mod header;
mod mem;
mod objects;
pub mod structs;
mod vtable;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use containers::{FName, FString};
use mem::{Ctx, CtxPtr, ExternalPtr, Mem, MemCache, NameTrait, StructsTrait};
use objects::FOptionalProperty;
use ordermap::OrderMap;
use patternsleuth_image::image::Image;
use patternsleuth_resolvers::{impl_try_collector, resolve};
use read_process_memory::{Pid, ProcessHandle};
use ue_reflection::{
    BytePropertyValue, Class, EClassCastFlags, Enum, EnumPropertyValue, Function, Object,
    ObjectType, Package, Property, PropertyType, PropertyValue, ReflectionData, ScriptStruct,
    Struct,
};

use crate::containers::PtrFNamePool;
use crate::objects::{
    FArrayProperty, FBoolProperty, FByteProperty, FClassProperty, FDelegateProperty, FEnumProperty,
    FInterfaceProperty, FLazyObjectProperty, FMapProperty, FMulticastDelegateProperty,
    FObjectProperty, FProperty, FSetProperty, FSoftClassProperty, FSoftObjectProperty,
    FStructProperty, FUObjectArray, FWeakObjectProperty, UClass, UEnum, UFunction, UObject,
    UScriptStruct, UStruct,
};
use crate::structs::Structs;

impl_try_collector! {
    #[derive(Debug, PartialEq, Clone)]
    struct Resolution {
        guobject_array: patternsleuth_resolvers::unreal::guobject_array::GUObjectArray,
        fname_pool: patternsleuth_resolvers::unreal::fname::FNamePool,
        engine_version: patternsleuth_resolvers::unreal::engine_version::EngineVersion,
    }
}

// TODO
// [ ] UStruct?
// [ ] interfaces
// [ ] functions signatures
// [ ] native function pointers
// [ ] dynamic structs
// [ ] ue version info

trait MemComplete: Mem + Clone + NameTrait + StructsTrait {}
impl<T: Mem + Clone + NameTrait + StructsTrait> MemComplete for T {}

fn read_path<M: MemComplete>(obj: &CtxPtr<UObject, M>) -> Result<String> {
    let mut objects = vec![obj.clone()];

    let mut obj = obj.clone();
    while let Some(outer) = obj.outer_private().read()? {
        objects.push(outer.clone());
        obj = outer;
    }

    let mut path = String::new();
    let mut prev: Option<&CtxPtr<UObject, M>> = None;
    for obj in objects.iter().rev() {
        if let Some(prev) = prev {
            let sep = if prev
                .class_private()
                .read()?
                .class_cast_flags()
                .read()?
                .contains(EClassCastFlags::CASTCLASS_UPackage)
            {
                '.'
            } else {
                ':'
            };
            path.push(sep);
        }
        path.push_str(&obj.name_private().read()?);
        prev = Some(obj);
    }

    Ok(path)
}

fn map_prop<M: MemComplete>(ptr: &CtxPtr<FProperty, M>) -> Result<Property> {
    let name = ptr.ffield().name_private().read()?;
    let field_class = ptr.ffield().class_private().read()?;
    let f = field_class.cast_flags().read()?;

    let t = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
        let prop = ptr.cast::<FStructProperty>();
        let s = prop.struct_().read()?.path()?;
        PropertyType::Struct { r#struct: s }
    } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
        PropertyType::Str
    } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
        PropertyType::Name
    } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
        PropertyType::Text
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
        let prop = ptr.cast::<FMulticastDelegateProperty>();
        let signature_function = prop.signature_function().read()?.path()?;
        PropertyType::MulticastInlineDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
        let prop = ptr.cast::<FMulticastDelegateProperty>();
        let signature_function = prop.signature_function().read()?.path()?;
        PropertyType::MulticastSparseDelegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
        let prop = ptr.cast::<FDelegateProperty>();
        let signature_function = prop.signature_function().read()?.path()?;
        PropertyType::Delegate { signature_function }
    } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
        let prop = ptr.cast::<FBoolProperty>();
        PropertyType::Bool {
            field_size: prop.field_size().read()?,
            byte_offset: prop.byte_offset_().read()?,
            byte_mask: prop.byte_mask().read()?,
            field_mask: prop.field_mask().read()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
        let prop = ptr.cast::<FArrayProperty>();
        PropertyType::Array {
            inner: map_prop(&prop.inner().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
        let prop = ptr.cast::<FEnumProperty>();
        PropertyType::Enum {
            container: map_prop(&prop.underlying_prop().read()?.cast())?.into(),
            r#enum: prop.enum_().read()?.map(|e| e.path()).transpose()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
        let prop = ptr.cast::<FMapProperty>();
        PropertyType::Map {
            key_prop: map_prop(&prop.key_prop().read()?.cast())?.into(),
            value_prop: map_prop(&prop.value_prop().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
        let prop = ptr.cast::<FSetProperty>();
        PropertyType::Set {
            key_prop: map_prop(&prop.element_prop().read()?.cast())?.into(),
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
        PropertyType::Float
    } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
        PropertyType::Double
    } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
        let prop = ptr.cast::<FByteProperty>();
        PropertyType::Byte {
            r#enum: prop.enum_().read()?.map(|e| e.path()).transpose()?,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
        PropertyType::UInt16
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
        PropertyType::UInt32
    } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
        PropertyType::UInt64
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
        PropertyType::Int8
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
        PropertyType::Int16
    } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
        PropertyType::Int
    } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
        PropertyType::Int64
    } else if f.contains(EClassCastFlags::CASTCLASS_FClassProperty) {
        let prop = ptr.cast::<FClassProperty>();
        let property_class = prop.fobject_property().property_class().read()?.path()?;
        let meta_class = prop.meta_class().read()?.path()?;
        PropertyType::Class {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
        let prop = ptr.cast::<FObjectProperty>();
        let property_class = prop.property_class().read()?.path()?;
        PropertyType::Object { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftClassProperty) {
        let prop = ptr.cast::<FSoftClassProperty>();
        let property_class = prop
            .fsoft_object_property()
            .property_class()
            .read()?
            .path()?;
        let meta_class = prop.meta_class().read()?.path()?;
        PropertyType::SoftClass {
            property_class,
            meta_class,
        }
    } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
        let prop = ptr.cast::<FSoftObjectProperty>();
        let property_class = prop.property_class().read()?.path()?;
        PropertyType::SoftObject { property_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
        let prop = ptr.cast::<FWeakObjectProperty>();
        let c = prop.property_class().read()?.path()?;
        PropertyType::WeakObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
        let prop = ptr.cast::<FLazyObjectProperty>();
        let c = prop.property_class().read()?.path()?;
        PropertyType::LazyObject { property_class: c }
    } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
        let prop = ptr.cast::<FInterfaceProperty>();
        let interface_class = prop.interface_class().read()?.path()?;
        PropertyType::Interface { interface_class }
    } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
        // TODO
        PropertyType::FieldPath
    } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
        let prop = ptr.cast::<FOptionalProperty>();
        PropertyType::Optional {
            inner: map_prop(&prop.value_property().read()?.cast())?.into(),
        }
    } else {
        unimplemented!("{f:?}");
    };

    let prop = ptr.cast::<FProperty>();
    Ok(Property {
        name,
        offset: prop.offset_internal().read()? as usize,
        array_dim: prop.array_dim().read()? as usize,
        size: prop.element_size().read()? as usize,
        flags: prop.property_flags().read()?,
        r#type: t,
    })
}

#[derive(Clone)]
struct ImgMem<'img, 'data>(&'img Image<'data>);

impl Mem for ImgMem<'_, '_> {
    fn read_buf(&self, address: usize, buf: &mut [u8]) -> Result<()> {
        self.0.memory.read(address, buf)?;
        Ok(())
    }
}

pub enum Input {
    Process(i32),
    Dump(PathBuf),
}

pub fn dump(input: Input, struct_info: Option<Structs>) -> Result<ReflectionData> {
    match input {
        Input::Process(pid) => {
            let handle: ProcessHandle = (pid as Pid).try_into()?;
            let mem = MemCache::wrap(handle);
            let image = patternsleuth_image::process::external::read_image_from_pid(pid)?;
            dump_inner(mem, &image, struct_info)
        }
        Input::Dump(path) => {
            let file = std::fs::File::open(path)?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

            let image = patternsleuth_image::image::Image::read::<&str>(None, &mmap, None, false)?;
            let mem = ImgMem(&image);
            dump_inner(mem, &image, struct_info)
        }
    }
}

use script_containers::*;
mod script_containers {
    use super::*;

    #[derive(Clone, Copy)]
    pub struct FScriptArray;
    impl<C: Clone + StructsTrait> CtxPtr<FScriptArray, C> {
        pub fn data(&self) -> CtxPtr<Option<ExternalPtr<()>>, C> {
            self.byte_offset(0).cast()
        }
        pub fn num(&self) -> CtxPtr<u32, C> {
            self.byte_offset(8).cast()
        }
    }
}

fn dump_inner<M: Mem + Clone>(
    mem: M,
    image: &Image<'_>,
    struct_info: Option<Structs>,
) -> Result<ReflectionData> {
    let results = resolve(image, Resolution::resolver())?;
    println!("{results:X?}");

    let guobjectarray = ExternalPtr::<FUObjectArray>::new(results.guobject_array.0);
    let fnamepool = PtrFNamePool(results.fname_pool.0);

    let struct_info = struct_info
        .or_else(|| structs::get_struct_info_for_version(&results.engine_version))
        .with_context(|| {
            format!(
                "Missing built-in struct info for {:?} please supply one with --struct_info or make an issue",
                results.engine_version
            )
        })?;

    let mem = Ctx {
        mem,
        fnamepool,
        structs: Arc::new(
            struct_info
                .0
                .into_iter()
                .map(|s| (s.name.clone(), s))
                .collect(),
        ),
    };

    let uobject_array = guobjectarray.ctx(mem);

    let mut objects = BTreeMap::<String, ObjectType>::default();
    let mut child_map = HashMap::<String, BTreeSet<String>>::default();

    for i in 0..uobject_array.obj_object().num_elements().read()? {
        let obj_item = uobject_array.obj_object().read_item_ptr(i as usize)?;
        let Some(obj) = obj_item.object().read()? else {
            continue;
        };
        let class = obj.class_private().read()?;

        let path = obj.path()?;

        fn for_each_prop<F, C: MemComplete>(ustruct: &CtxPtr<UStruct, C>, mut f: F) -> Result<()>
        where
            F: FnMut(&CtxPtr<FProperty, C>) -> Result<()>,
        {
            let mut str = Some(ustruct.clone());
            while let Some(next_struct) = str {
                let mut field = next_struct.child_properties();
                while let Some(next) = field.read()? {
                    let flags = next.class_private().read()?.cast_flags().read()?;
                    if flags.contains(EClassCastFlags::CASTCLASS_FProperty) {
                        f(&next.cast::<FProperty>())?;
                    }
                    field = next.next();
                }
                str = next_struct.super_struct().read()?;
            }
            Ok(())
        }

        fn read_props<M: MemComplete>(
            ustruct: &CtxPtr<UStruct, M>,
            ptr: &CtxPtr<(), M>,
        ) -> Result<OrderMap<String, PropertyValue>> {
            let mut properties = OrderMap::new();
            for_each_prop(&ustruct, |prop| {
                let array_dim = prop.array_dim().read()? as usize;
                let name = prop.ffield().name_private().read()?;
                if array_dim == 1 {
                    if let Some(value) = read_prop(prop, &ptr, 0)? {
                        properties.insert(name, value);
                    }
                } else {
                    let mut elements = vec![];
                    for i in 0..array_dim {
                        if let Some(value) = read_prop(prop, &ptr, i)? {
                            elements.push(value);
                        } else {
                            return Ok(());
                        }
                    }
                    properties.insert(name, PropertyValue::Array(elements));
                }
                Ok(())
            })?;
            Ok(properties)
        }
        fn read_prop<M: MemComplete>(
            prop: &CtxPtr<FProperty, M>,
            ptr: &CtxPtr<(), M>,
            index: usize,
        ) -> Result<Option<PropertyValue>> {
            let size = prop.element_size().read()? as usize;
            let ptr = ptr.byte_offset(prop.offset_internal().read()? as usize + index * size);
            let f = prop.ffield().class_private().read()?.cast_flags().read()?;

            let value = if f.contains(EClassCastFlags::CASTCLASS_FStructProperty) {
                let prop = prop.cast::<FStructProperty>();
                PropertyValue::Struct(read_props(&prop.struct_().read()?.ustruct(), &ptr)?)
            } else if f.contains(EClassCastFlags::CASTCLASS_FStrProperty) {
                PropertyValue::Str(ptr.cast::<FString>().read()?)
            } else if f.contains(EClassCastFlags::CASTCLASS_FNameProperty) {
                PropertyValue::Name(ptr.cast::<FName>().read()?)
            } else if f.contains(EClassCastFlags::CASTCLASS_FTextProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastInlineDelegateProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FMulticastSparseDelegateProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FDelegateProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FBoolProperty) {
                let prop = prop.cast::<FBoolProperty>();
                let byte_offset = prop.byte_offset_().read()?;
                let byte_mask = prop.byte_mask().read()?;
                let byte = ptr.byte_offset(byte_offset as usize).cast::<u8>().read()?;
                PropertyValue::Bool(byte & byte_mask != 0)
            } else if f.contains(EClassCastFlags::CASTCLASS_FArrayProperty) {
                let prop = prop.cast::<FArrayProperty>();
                let array = ptr.cast::<FScriptArray>();

                let num = array.num().read()? as usize;
                let mut data = Vec::with_capacity(num);
                if let Some(data_ptr) = array.data().read()? {
                    let inner_prop = prop.inner().read()?;
                    for i in 0..num {
                        // TODO handle size != alignment
                        let value = read_prop(&inner_prop, &data_ptr, i)?;
                        if let Some(value) = value {
                            data.push(value);
                        } else {
                            return Ok(None);
                        }
                    }
                }

                PropertyValue::Array(data)
            } else if f.contains(EClassCastFlags::CASTCLASS_FEnumProperty) {
                let prop = prop.cast::<FEnumProperty>();
                let underlying = read_prop(&prop.underlying_prop().read()?, &ptr, 0)?
                    .expect("valid underlying prop");
                let value = match underlying {
                    PropertyValue::Byte(BytePropertyValue::Value(v)) => v as i64,
                    PropertyValue::Int8(v) => v as i64,
                    PropertyValue::Int16(v) => v as i64,
                    PropertyValue::Int(v) => v as i64,
                    PropertyValue::UInt16(v) => v as i64,
                    PropertyValue::UInt32(v) => v as i64,
                    e => unimplemented!("underlying enum prop {e:#?}"),
                };
                let names = read_enum(&prop.enum_().read()?.expect("valid enum"))?.names;
                let name = names
                    .into_iter()
                    .find_map(|(name, v)| (v == value).then_some(name));

                PropertyValue::Enum(if let Some(name) = name {
                    EnumPropertyValue::Name(name)
                } else {
                    EnumPropertyValue::Value(value)
                })
            } else if f.contains(EClassCastFlags::CASTCLASS_FMapProperty) {
                // /* offset 0x000 */ Data: TScriptArray<TSizedDefaultAllocator<32> >,
                // /* offset 0x010 */ AllocationFlags: TScriptBitArray<FDefaultBitArrayAllocator,void>,
                // /* offset 0x030 */ FirstFreeIndex: i32,
                // /* offset 0x034 */ NumFreeIndices: i32,

                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FSetProperty) {
                //let prop = prop.cast::<FSetProperty>();
                //#[derive(Clone, Copy)]
                //pub struct FScriptSet;
                //impl<C: Clone + StructsTrait> CtxPtr<FScriptSet, C> {
                //    pub fn data(&self) -> CtxPtr<FScriptArray, C> {
                //        self.byte_offset(0).cast()
                //    }
                //    pub fn allocation_flags(&self) -> CtxPtr<TBitArray<TInlineAllocator<4>>, C> {
                //        self.byte_offset(16).cast()
                //    }
                //}
                //let array = ptr.cast::<FScriptSet>();
                //dbg!(array.allocation_flags().read()?);
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FFloatProperty) {
                PropertyValue::Float(ptr.cast::<f32>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FDoubleProperty) {
                PropertyValue::Double(ptr.cast::<f64>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FByteProperty) {
                let prop = prop.cast::<FByteProperty>();
                let value = ptr.cast::<u8>().read()?;
                PropertyValue::Byte(
                    if let Some(name) = prop
                        .enum_()
                        .read()?
                        .map(|e| read_enum(&e))
                        .transpose()?
                        .and_then(|e| {
                            e.names
                                .into_iter()
                                .find_map(|(name, v)| (v == value as i64).then_some(name))
                        })
                    {
                        BytePropertyValue::Name(name)
                    } else {
                        BytePropertyValue::Value(value)
                    },
                )
            } else if f.contains(EClassCastFlags::CASTCLASS_FUInt16Property) {
                PropertyValue::UInt16(ptr.cast::<u16>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FUInt32Property) {
                PropertyValue::UInt32(ptr.cast::<u32>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FUInt64Property) {
                PropertyValue::UInt64(ptr.cast::<u64>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FInt8Property) {
                PropertyValue::Int8(ptr.cast::<i8>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FInt16Property) {
                PropertyValue::Int16(ptr.cast::<i16>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FIntProperty) {
                PropertyValue::Int(ptr.cast::<i32>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FInt64Property) {
                PropertyValue::Int64(ptr.cast::<i64>().read()?.into())
            } else if f.contains(EClassCastFlags::CASTCLASS_FObjectProperty) {
                let obj = ptr
                    .cast::<Option<ExternalPtr<UObject>>>()
                    .read()?
                    .map(|e| e.path())
                    .transpose()?;
                PropertyValue::Object(obj)
            } else if f.contains(EClassCastFlags::CASTCLASS_FWeakObjectProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FSoftObjectProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FLazyObjectProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FInterfaceProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FFieldPathProperty) {
                return Ok(None);
            } else if f.contains(EClassCastFlags::CASTCLASS_FOptionalProperty) {
                return Ok(None);
            } else {
                unimplemented!("{f:?}");
            };
            Ok(Some(value))
        }

        fn read_object<M: MemComplete>(obj: &CtxPtr<UObject, M>) -> Result<Object> {
            let outer = obj.outer_private().read()?.map(|s| s.path()).transpose()?;

            let class = obj.class_private().read()?;
            let class_name = class.path()?;

            Ok(Object {
                vtable: obj.vtable().read()? as u64,
                object_flags: obj.object_flags().read()?,
                outer,
                class: class_name,
                children: Default::default(),
                property_values: read_props(&class.ustruct(), &obj.cast())?.into(),
            })
        }

        fn read_struct<M: MemComplete>(obj: &CtxPtr<UStruct, M>) -> Result<Struct> {
            let mut properties = vec![];
            let mut field = obj.child_properties();
            while let Some(next) = field.read()? {
                let f = next.class_private().read()?.cast_flags().read()?;
                if f.contains(EClassCastFlags::CASTCLASS_FProperty) {
                    properties.push(map_prop(&next.cast::<FProperty>())?);
                }

                field = next.next();
            }
            let super_struct = obj.super_struct().read()?.map(|s| s.path()).transpose()?;
            Ok(Struct {
                object: read_object(&obj.cast())?,
                super_struct,
                properties,
                properties_size: obj.properties_size().read()? as usize,
                min_alignment: obj.min_alignment().read()? as usize,
            })
        }

        fn read_script_struct<M: MemComplete>(
            obj: &CtxPtr<UScriptStruct, M>,
        ) -> Result<ScriptStruct> {
            Ok(ScriptStruct {
                r#struct: read_struct(&obj.ustruct())?,
                struct_flags: obj.struct_flags().read()?,
            })
        }

        fn read_class<M: MemComplete>(obj: &CtxPtr<UClass, M>) -> Result<Class> {
            let class_flags = obj.class_flags().read()?;
            let class_cast_flags = obj.class_cast_flags().read()?;
            let class_default_object = obj
                .class_default_object()
                .read()?
                .map(|s| s.path())
                .transpose()?;
            Ok(Class {
                r#struct: read_struct(&obj.cast())?,
                class_flags,
                class_cast_flags,
                class_default_object,
                instance_vtable: None,
            })
        }

        fn read_enum<M: MemComplete>(obj: &CtxPtr<UEnum, M>) -> Result<Enum> {
            let mut names = vec![];
            for item in obj.names().iter()? {
                let key = item.a().read()?;
                let value = item.b().read()?;
                names.push((key, value));
            }
            Ok(Enum {
                object: read_object(&obj.cast())?,
                cpp_type: obj.cpp_type().read()?,
                cpp_form: obj.cpp_form().read()?,
                enum_flags: obj.enum_flags().read()?,
                names,
            })
        }

        if !path.starts_with("/Script/") {
            continue;
        }
        let f = class.class_cast_flags().read()?;
        let object = if f.contains(EClassCastFlags::CASTCLASS_UClass) {
            ObjectType::Class(read_class(&obj.cast())?)
        } else if f.contains(EClassCastFlags::CASTCLASS_UFunction) {
            let full_obj = obj.cast::<UFunction>();
            let function_flags = full_obj.function_flags().read()?;
            ObjectType::Function(Function {
                r#struct: read_struct(&obj.cast())?,
                function_flags,
                func: full_obj.func().read()? as u64,
            })
        } else if f.contains(EClassCastFlags::CASTCLASS_UScriptStruct) {
            ObjectType::ScriptStruct(read_script_struct(&obj.cast())?)
        } else if f.contains(EClassCastFlags::CASTCLASS_UEnum) {
            ObjectType::Enum(read_enum(&obj.cast())?)
        } else if f.contains(EClassCastFlags::CASTCLASS_UPackage) {
            ObjectType::Package(Package {
                object: read_object(&obj)?,
            })
        } else {
            let obj = obj.cast::<UObject>();
            ObjectType::Object(read_object(&obj)?)
            //println!("{path:?} {:?}", f);
        };

        // update child_map
        {
            if let Some(outer) = object.get_object().outer.clone() {
                child_map.entry(outer).or_default().insert(path.clone());
            }
        }

        objects.insert(path, object);
    }

    for (outer, children) in child_map {
        match objects.get_mut(&outer).unwrap() {
            ObjectType::Package(obj) => &mut obj.object,
            ObjectType::Enum(obj) => &mut obj.object,
            ObjectType::ScriptStruct(obj) => &mut obj.r#struct.object,
            ObjectType::Class(obj) => &mut obj.r#struct.object,
            ObjectType::Function(obj) => &mut obj.r#struct.object,
            ObjectType::Object(obj) => obj,
        }
        .children = children;
    }

    let vtables = vtable::analyze_vtables(image, &mut objects);

    Ok(ReflectionData {
        image_base_address: image.base_address as u64,
        objects,
        vtables,
    })
}

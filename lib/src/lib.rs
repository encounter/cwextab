#![no_std]
extern crate alloc;

use alloc::string::String;
use alloc::{format, vec};
use alloc::vec::Vec;
use thiserror::Error;

mod mem_utils;

#[derive(Error, Debug)]
pub enum ExtabDecodeError {
    #[error("Data array should at least be 8 bytes long. Given array is {0} bytes long.")]
    ArrayTooSmall(u32),
    #[error("Invalid action value {0} at offset 0x{1:X}")]
    InvalidActionValue(u32, u32),
    #[error("Table is 8 bytes long but terminator is not zero.")]
    InvalidSmallTableTerminator,
    #[error("Internal error")]
    Internal,
}

/// Enum holding the data for each action type.
#[derive(Debug, Clone)]
pub enum ExActionData {
    EndOfList,
    Branch {
        target_offset: u16,
    },
    DestroyLocal {
        local_offset: u16,
        dtor_address: u32,
    },
    DestroyLocalCond {
        condition: u16,
        local_offset: u16,
        unk4: u16,
        dtor_address: u32,
    },
    DestroyLocalPointer {
        local_pointer: u16,
        dtor_address: u32,
    },
    DestroyLocalArray {
        local_array: u16,
        elements: u16,
        element_size: u16,
        dtor_address: u32,
    },
    DestroyBase {
        object_pointer: u16,
        member_offset: u32,
        dtor_address: u32,
    },
    DestroyMember {
        object_pointer: u16,
        member_offset: u32,
        dtor_address: u32,
    },
    DestroyMemberCond {
        condition: u16,
        object_pointer: u16,
        member_offset: u32,
        unk8: u16,
        dtor_address: u32,
    },
    DestroyMemberArray {
        object_pointer: u16,
        member_offset: u32,
        elements: u32,
        element_size: u32,
        dtor_address: u32,
    },
    DeletePointer {
        object_pointer: u16,
        dtor_address: u32,
    },
    DeletePointerCond {
        condition: u16,
        object_pointer: u16,
        unk4: u16,
        dtor_address: u32,
    },
    CatchBlock {
        unk0: u16,
        catch_type: u32,
        catch_pc_offset: u16,
        cinfo_ref: u16,
    },
    ActiveCatchBlock {
        cinfo_ref: u16,
    },
    Terminate,
    Specification {
        specs: u16,
        pc_offset: u32,
        cinfo_ref: u32,
        spec: Vec<u32>,
    },
    CatchBlock32 {
        unk0: u16,
        catch_type: u32,
        catch_pc_offset: u32,
        cinfo_ref: u32,
    },
}

/// Base enum for exception actions.
#[derive(Debug, Copy, Clone)]
pub enum ExAction {
    EndOfList,
    Branch,
    DestroyLocal,
    DestroyLocalCond,
    DestroyLocalPointer,
    DestroyLocalArray,
    DestroyBase,
    DestroyMember,
    DestroyMemberCond,
    DestroyMemberArray,
    DeletePointer,
    DeletePointerCond,
    CatchBlock,
    ActiveCatchBlock,
    Terminate,
    Specification,
    CatchBlock32,
}

impl ExAction {
    pub fn to_int(&self) -> i32 {
        match self {
            ExAction::EndOfList => 0,
            ExAction::Branch => 1,
            ExAction::DestroyLocal => 2,
            ExAction::DestroyLocalCond => 3,
            ExAction::DestroyLocalPointer => 4,
            ExAction::DestroyLocalArray => 5,
            ExAction::DestroyBase => 6,
            ExAction::DestroyMember => 7,
            ExAction::DestroyMemberCond => 8,
            ExAction::DestroyMemberArray => 9,
            ExAction::DeletePointer => 10,
            ExAction::DeletePointerCond => 11,
            ExAction::CatchBlock => 12,
            ExAction::ActiveCatchBlock => 13,
            ExAction::Terminate => 14,
            ExAction::Specification => 15,
            ExAction::CatchBlock32 => 16,
        }
    }

    pub fn from_int(val: i32) -> Option<ExAction> {
        let result: ExAction = match val {
            0 => ExAction::EndOfList,
            1 => ExAction::Branch,
            2 => ExAction::DestroyLocal,
            3 => ExAction::DestroyLocalCond,
            4 => ExAction::DestroyLocalPointer,
            5 => ExAction::DestroyLocalArray,
            6 => ExAction::DestroyBase,
            7 => ExAction::DestroyMember,
            8 => ExAction::DestroyMemberCond,
            9 => ExAction::DestroyMemberArray,
            10 => ExAction::DeletePointer,
            11 => ExAction::DeletePointerCond,
            12 => ExAction::CatchBlock,
            13 => ExAction::ActiveCatchBlock,
            14 => ExAction::Terminate,
            15 => ExAction::Specification,
            16 => ExAction::CatchBlock32,
            _ => {
                //The action value is invalid, return None
                return None;
            }
        };
        Some(result)
    }

    const ACTION_NAMES: [&'static str; 17] = [
        "NULL",
        "BRANCH",
        "DESTROYLOCAL",
        "DESTROYLOCALCOND",
        "DESTROYLOCALPOINTER",
        "DESTROYLOCALARRAY",
        "DESTROYBASE",
        "DESTROYMEMBER",
        "DESTROYMEMBERCOND",
        "DESTROYMEMBERARRAY",
        "DELETEPOINTER",
        "DELETEPOINTERCOND",
        "CATCHBLOCK (Small)",
        "ACTIVECATCHBLOCK",
        "TERMINATE",
        "SPECIFICATION",
        "CATCHBLOCK (Large)",
    ];

    fn convert_to_string(&self) -> String {
        String::from(Self::ACTION_NAMES[self.to_int() as usize])
    }
}

/// Struct for exception actions.
#[derive(Debug, Clone)]
pub struct ExceptionAction {
    //General values
    pub action_offset: u32,
    pub action_type: ExAction, //0x0
    pub action_param: u8,      //0x1
    pub has_end_bit: bool,     //true if action type byte has bit 7 set (type & 0x80)
    pub bytes: Vec<u8>,
}

impl ExceptionAction {
    pub fn new() -> Self {
        Self {
            action_offset: 0,
            action_type: ExAction::EndOfList,
            action_param: 0,
            has_end_bit: false,
            bytes: vec![],
        }
    }

    /// Returns whether this action has a destuctor reference or not.
    pub fn has_dtor_ref(&self) -> bool {
        match self.action_type {
            ExAction::EndOfList
            | ExAction::Branch
            | ExAction::CatchBlock
            | ExAction::ActiveCatchBlock
            | ExAction::Terminate
            | ExAction::Specification
            | ExAction::CatchBlock32 => false,
            _ => true,
        }
    }

    /// Calculates the offset of the dtor function address value in this action entry.
    /// If the entry does not have one, this function returns none.
    fn get_dtor_address_value_offset(&self) -> Option<u32> {
        let offset: u32 =
        match self.action_type {
            ExAction::DestroyLocal => 2,
            ExAction::DestroyLocalCond => 6,
            ExAction::DestroyLocalPointer => 2,
            ExAction::DestroyLocalArray => 6,
            ExAction::DestroyBase
            | ExAction::DestroyMember => 6,
            ExAction::DestroyMemberCond => 10,
            ExAction::DestroyMemberArray => 14,
            ExAction::DeletePointer => 2,
            ExAction::DeletePointerCond => 6,
            _ => return None,
        };

        Some(offset)
    }

    /// Returns the relocation data for the dtor function in this action entry, if any.
    pub fn get_dtor_relocation(&self) -> Option<(u32, u32)> {
        if !self.has_dtor_ref() {
            //If the action entry doesn't have a dtor reference, return none
            return None;
        }

        let offset: u32 = match self.get_dtor_address_value_offset() {
            Some(val) => val,
            None => {
                return None;
            }
        };

        let address: u32 = mem_utils::read_uint32(&self.bytes, &mut (offset as i32), true);
        Some((offset, address))
    }

    /// Decodes the action data from the byte array depending on the set action type, and converts it
    /// to an ExActionData enum containing the decoded data.
    pub fn get_exaction_data(&self) -> ExActionData {
        let mut offset: i32 = 0;

        match self.action_type {
            ExAction::EndOfList => ExActionData::EndOfList {},
            ExAction::Branch => {
                let target_offset = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                ExActionData::Branch { target_offset }
            }
            ExAction::DestroyLocal => {
                let local_offset = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyLocal {
                    local_offset,
                    dtor_address,
                }
            }
            ExAction::DestroyLocalCond => {
                let condition = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let local_offset = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let unk4 = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyLocalCond {
                    condition,
                    local_offset,
                    unk4,
                    dtor_address,
                }
            }
            ExAction::DestroyLocalPointer => {
                let local_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyLocalPointer {
                    local_pointer,
                    dtor_address,
                }
            }
            ExAction::DestroyLocalArray => {
                let local_array = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let elements = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let element_size = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyLocalArray {
                    local_array,
                    elements,
                    element_size,
                    dtor_address,
                }
            }
            ExAction::DestroyBase => {
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let member_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyBase {
                    object_pointer,
                    member_offset,
                    dtor_address,
                }
            }
            ExAction::DestroyMember => {
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let member_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyMember {
                    object_pointer,
                    member_offset,
                    dtor_address,
                }
            }
            ExAction::DestroyMemberCond => {
                let condition = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let member_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let unk8 = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyMemberCond {
                    condition,
                    object_pointer,
                    member_offset,
                    unk8,
                    dtor_address,
                }
            }
            ExAction::DestroyMemberArray => {
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let member_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let elements = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let element_size = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DestroyMemberArray {
                    object_pointer,
                    member_offset,
                    elements,
                    element_size,
                    dtor_address,
                }
            }
            ExAction::DeletePointer => {
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DeletePointer {
                    object_pointer,
                    dtor_address,
                }
            }
            ExAction::DeletePointerCond => {
                let condition = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let object_pointer = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let unk4 = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let dtor_address = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::DeletePointerCond {
                    condition,
                    object_pointer,
                    unk4,
                    dtor_address,
                }
            }
            ExAction::CatchBlock => {
                let unk0 = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let catch_type = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let catch_pc_offset = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let cinfo_ref = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                ExActionData::CatchBlock {
                    unk0,
                    catch_type,
                    catch_pc_offset,
                    cinfo_ref,
                }
            }
            ExAction::ActiveCatchBlock => {
                let cinfo_ref = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                ExActionData::ActiveCatchBlock { cinfo_ref }
            }
            ExAction::Terminate => ExActionData::Terminate {},
            ExAction::Specification => {
                let specs = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let pc_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let cinfo_ref = mem_utils::read_uint32(&self.bytes, &mut offset, true);

                //Read the specified number of 32 bit values and add them to the list
                let length = specs as i32;
                let mut spec: Vec<u32> = vec![];
                for _i in 0..length {
                    spec.push(mem_utils::read_uint32(&self.bytes, &mut offset, true));
                }
                ExActionData::Specification {
                    specs,
                    pc_offset,
                    cinfo_ref,
                    spec,
                }
            }
            ExAction::CatchBlock32 => {
                let unk0 = mem_utils::read_uint16(&self.bytes, &mut offset, true);
                let catch_type = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let catch_pc_offset = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                let cinfo_ref = mem_utils::read_uint32(&self.bytes, &mut offset, true);
                ExActionData::CatchBlock32 {
                    unk0,
                    catch_type,
                    catch_pc_offset,
                    cinfo_ref,
                }
            }
        }
    }
}

impl Default for ExceptionAction {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct for pc actions.
#[derive(Debug, Clone)]
pub struct PCAction {
    pub start_pc: u32,
    pub end_pc: u32,
    pub action_offset: u32,
}

impl PCAction {
    pub fn new() -> Self {
        Self {
            start_pc: 0,
            end_pc: 0,
            action_offset: 0,
        }
    }
}

impl Default for PCAction {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct for exception table relocation (always dtor function address)
#[derive(Debug, Clone)]
pub struct Relocation {
    pub offset: u32,
    pub address: u32,
}

/// Struct containing all the data from the decoded exception table.
#[derive(Debug, Clone)]
pub struct ExceptionTableData {
    pub flag_val: u16, //0x0-1
    //Flag bits (16 bit value)
    pub has_elf_vector: bool,    //bit 1
    pub large_frame: bool,       //bit 3
    pub has_frame_pointer: bool, //bit 4
    pub saved_cr: bool,          //bit 5
    pub fpr_save_range: u32,     //bits 6-10
    pub gpr_save_range: u32,     //bits 11-15

    pub et_field: u16, //0x2-3

    pub pc_actions: Vec<PCAction>,
    pub exception_actions: Vec<ExceptionAction>,
    pub relocations: Vec<Relocation>,
}

impl ExceptionTableData {
    fn new() -> Self {
        Self {
            flag_val: 0,
            has_elf_vector: false,
            large_frame: false,
            has_frame_pointer: false,
            saved_cr: false,
            fpr_save_range: 0,
            gpr_save_range: 0,
            et_field: 0,
            pc_actions: vec![],
            exception_actions: vec![],
            relocations: vec![],
        }
    }

    fn calculate_flag_values(&mut self) {
        self.has_elf_vector = ((self.flag_val >> 1) & 1) == 1;
        self.large_frame = ((self.flag_val >> 3) & 1) == 1;
        self.has_frame_pointer = ((self.flag_val >> 4) & 1) == 1;
        self.saved_cr = ((self.flag_val >> 5) & 1) == 1;
        self.fpr_save_range = ((self.flag_val >> 6) & 0b11111) as u32;
        self.gpr_save_range = ((self.flag_val >> 11) & 0b11111) as u32;
    }

    /// Converts the table into a string, taking in an array of the function
    /// names required for the table.
    ///
    /// Returns 'None' if an error occurs.
    pub fn to_string(&self, func_names: Vec<String>) -> Option<String> {
        let mut sb = String::from("");

        sb += "Flag values:\n";
        sb += format!(
            "{}",
            format_args!(
                "Has Elf Vector: {}\n",
                if self.has_elf_vector { "Yes" } else { "No" }
            )
        )
        .as_str();
        sb += format!(
            "{}",
            format_args!(
                "Large Frame: {}\n",
                if self.large_frame { "Yes" } else { "No" }
            )
        )
        .as_str();
        sb += format!(
            "{}",
            format_args!(
                "Has Frame Pointer: {}\n",
                if self.has_frame_pointer { "Yes" } else { "No" }
            )
        )
        .as_str();
        sb += format!(
            "{}",
            format_args!("Saved CR: {}\n", if self.saved_cr { "Yes" } else { "No" })
        )
        .as_str();

        if self.fpr_save_range != 0 {
            let start_fpr = 31 - (self.fpr_save_range - 1);
            let fpr_string: String = if start_fpr == 31 {
                String::from("fp31")
            } else {
                format!("fp{start_fpr}-fp31")
            };
            sb += format!("Saved FPR range: {fpr_string}\n").as_str();
        }
        if self.gpr_save_range != 0 {
            let start_gpr = 31 - (self.gpr_save_range - 1);
            let gpr_string: String = if start_gpr == 31 {
                String::from("r31")
            } else {
                format!("r{start_gpr}-r31")
            };
            sb += format!("Saved GPR range: {gpr_string}\n").as_str();
        }
        sb += "\n";

        let num_pcactions = self.pc_actions.len();

        //Print exception range entries
        if num_pcactions > 0 {
            sb += "PC actions:\n";
            for i in 0..num_pcactions {
                let action = &self.pc_actions[i];
                let start_pc = action.start_pc;
                let end_pc = action.end_pc;
                let action_offset = action.action_offset;
                if start_pc != end_pc {
                    sb += format!("PC={start_pc:08X}:{end_pc:08X}, Action: {action_offset:06X}\n")
                        .as_str();
                } else {
                    sb += format!("PC={start_pc:08X}, Action: {action_offset:06X}\n").as_str();
                }
            }

            sb += "\n";
        }

        let num_exactions = self.exception_actions.len();

        if num_exactions > 0 {
            sb += "Exception actions:\n";
            let local_reg_string = if self.has_frame_pointer { "FP" } else { "SP" };
            let mut func_index: usize = 0;

            for i in 0..num_exactions {
                let action = &self.exception_actions[i];
                let mut line = String::from("");
                let action_offset = action.action_offset;
                let action_name = action.action_type.convert_to_string();
                line += format!("{action_offset:06X}:\nType: {action_name}\n").as_str();

                let has_dtor_ref = action.has_dtor_ref();
                let exaction_data = action.get_exaction_data();

                match exaction_data {
                    ExActionData::EndOfList => {}
                    ExActionData::Branch { target_offset } => {
                        line += format!("Action: {target_offset:06X}\n").as_str();
                    }
                    ExActionData::DestroyLocal { local_offset, .. } => {
                        line += format!("Local: {local_offset:#X}({local_reg_string})\n").as_str();
                    }
                    ExActionData::DestroyLocalCond {
                        condition,
                        local_offset,
                        ..
                    } => {
                        line += format!("Local: {local_offset:#X}({local_reg_string})\n").as_str();

                        //The action param is used to determine the type of reference for the condition (0: local offset, 1: register)
                        if action.action_param == 0 {
                            //Local offset
                            line += format!("Cond: {condition:#X}({local_reg_string})\n").as_str();
                        } else {
                            //Register
                            //In this case, the local offset param is actually the register number
                            line += format!("Cond: r{condition}\n").as_str();
                        }
                    }
                    ExActionData::DestroyLocalPointer { local_pointer, .. } => {
                        let mode = action.action_param >> 7;
                        if mode == 0 {
                            //Local offset
                            line +=
                                format!("Pointer: {local_pointer:#X}({local_reg_string})\n").as_str();
                        } else {
                            //Register
                            line += format!("Pointer: r{local_pointer}\n").as_str();
                        }
                    }
                    ExActionData::DestroyLocalArray {
                        local_array,
                        elements,
                        element_size,
                        ..
                    } => {
                        line += format!("Array: {local_array:#X}({local_reg_string})\nElements: {elements}\nSize: {element_size}\n").as_str();
                    }
                    ExActionData::DestroyBase {
                        object_pointer,
                        member_offset,
                        ..
                    } => {
                        let mode = action.action_param >> 7;
                        if mode == 0 {
                            line += format!("Member: {object_pointer:#X}({local_reg_string})+{member_offset:#X}\n").as_str();
                        } else {
                            line +=
                                format!("Member: {member_offset:#X}(r{object_pointer})\n").as_str();
                        }
                    }
                    ExActionData::DestroyMember {
                        object_pointer,
                        member_offset,
                        ..
                    } => {
                        let mode = action.action_param >> 7;
                        if mode == 0 {
                            line += format!("Member: {object_pointer:#X}({local_reg_string})+{member_offset:#X}\n").as_str();
                        } else {
                            line +=
                                format!("Member: {member_offset:#X}(r{object_pointer})\n").as_str();
                        }
                    }
                    ExActionData::DestroyMemberCond {
                        condition,
                        object_pointer,
                        member_offset,
                        ..
                    } => {
                        let mode = (action.action_param >> 6) & 1;
                        if mode == 0 {
                            line += format!("Member: {object_pointer:#X}({local_reg_string})+{member_offset:#X}\n").as_str();
                        } else {
                            //Register
                            line +=
                                format!("Member: {member_offset:#X}(r{object_pointer})\n").as_str();
                        }
                        let condition_mode = action.action_param >> 7;
                        if condition_mode == 0 {
                            //Local offset
                            line += format!("Cond: {condition:#X}({local_reg_string})\n").as_str();
                        } else {
                            //Register
                            line += format!("Cond: r{condition}\n").as_str();
                        }
                    }
                    ExActionData::DestroyMemberArray {
                        object_pointer,
                        member_offset,
                        elements,
                        element_size,
                        ..
                    } => {
                        let mode = action.action_param >> 7;
                        if mode == 0 {
                            //Local offset
                            line += format!(
                                "Member: {object_pointer:#X}({local_reg_string})+0x{member_offset}\n"
                            )
                            .as_str();
                        } else {
                            //Register
                            line +=
                                format!("Member: {member_offset:#X}(r{object_pointer})\n").as_str();
                        }
                        line += format!("Elements: {elements}\nSize: {element_size}\n").as_str();
                    }
                    ExActionData::DeletePointer { object_pointer, .. } => {
                        let mode = action.action_param >> 7;
                        if mode == 0 {
                            //Local offset
                            line += format!("Pointer: {object_pointer:#X}({local_reg_string})\n")
                                .as_str();
                        } else {
                            //Register
                            line += format!("Pointer: r{object_pointer})\n").as_str();
                        }
                    }
                    ExActionData::DeletePointerCond {
                        condition,
                        object_pointer,
                        ..
                    } => {
                        let mode = (action.action_param >> 6) & 1;
                        if mode == 0 {
                            //Local offset
                            line += format!("Pointer: {object_pointer:#X}({local_reg_string})\n")
                                .as_str();
                        } else {
                            //Register
                            line += format!("Pointer: r{object_pointer})\n").as_str();
                        }
                        let condition_mode = action.action_param >> 7;
                        if condition_mode == 0 {
                            //Local offset
                            line += format!("Cond: {condition:#X}({local_reg_string})\n").as_str();
                        } else {
                            //Register
                            line += format!("Cond: r{condition}\n").as_str();
                        }
                    }
                    ExActionData::CatchBlock {
                        catch_type,
                        catch_pc_offset,
                        cinfo_ref,
                        ..
                    } => {
                        line += format!("Local: {cinfo_ref:#X}({local_reg_string})\nPC: {catch_pc_offset:08X}\ncatch_type_addr: {catch_type:08X}\n").as_str();
                    }
                    ExActionData::ActiveCatchBlock { cinfo_ref } => {
                        line += format!("Local: {cinfo_ref:#X}({local_reg_string})\n").as_str();
                    }
                    ExActionData::Terminate => {}
                    ExActionData::Specification {
                        specs,
                        pc_offset,
                        cinfo_ref,
                        ..
                    } => {
                        line += format!("Local: {cinfo_ref:#X}({local_reg_string})\nPC: {pc_offset:08X}\nTypes: {specs}\n").as_str();
                    }
                    ExActionData::CatchBlock32 {
                        catch_type,
                        catch_pc_offset,
                        cinfo_ref,
                        ..
                    } => {
                        line += format!("Local: {cinfo_ref:#X}({local_reg_string})\nPC: {catch_pc_offset:08X}\ncatch_type_addr: {catch_type:08X}\n").as_str();
                    }
                }

                //If the action references a dtor, print it out using the name array
                if has_dtor_ref {
                    if func_index >= func_names.len() {
                        line += "Error: Invalid function array index\n";
                    } else {
                        let func_name = func_names[func_index].as_str();
                        line += format!("Dtor: \"{func_name}\"\n").as_str();
                        func_index += 1;
                    }
                }

                if action.has_end_bit {
                    line += "Has end bit\n"
                };
                sb += line.as_str(); //Print the line
            }
        }

        Some(sb)
    }
}

struct ExtabDecoder {
    extab_data: ExceptionTableData,
    offset: i32,
    data: Vec<u8>,
    length: i32,
}

impl ExtabDecoder {
    fn new() -> Self {
        Self {
            extab_data: ExceptionTableData::new(),
            offset: 0,
            data: vec![],
            length: 0,
        }
    }

    fn parse_exception_table(&mut self, bytes: &[u8]) -> Result<(), ExtabDecodeError> {
        self.offset = 0;
        self.data = Vec::from(bytes);
        self.length = self.data.len() as i32;

        //If the array is empty, return an error.
        if self.length < 8 {
            return Err(ExtabDecodeError::ArrayTooSmall(self.length as u32));
        }

        //Parse the header flag value
        self.extab_data.flag_val = mem_utils::read_uint16(&self.data, &mut self.offset, true);
        self.extab_data.calculate_flag_values();
        self.extab_data.et_field = mem_utils::read_uint16(&self.data, &mut self.offset, true);

        //Check whether the table is 8 bytes but the terminator isn't zero. If so,
        //throw an error.
        let terminator = mem_utils::read_uint32(&self.data, &mut self.offset, false);
        if self.length == 8 && terminator != 0 {
            return Err(ExtabDecodeError::InvalidSmallTableTerminator);
        }

        //Parse range entries until we hit the terminator (32 bit zero value)
        while mem_utils::read_uint32(&self.data, &mut self.offset, false) != 0 {
            let mut pcaction = PCAction::new();
            pcaction.start_pc = mem_utils::read_uint32(&self.data, &mut self.offset, true);
            let range_size: u32 =
                (mem_utils::read_uint16(&self.data, &mut self.offset, true) as u32) * 4; //range size is encoded as size >> 2
            pcaction.end_pc = pcaction.start_pc + range_size;
            pcaction.action_offset =
                mem_utils::read_uint16(&self.data, &mut self.offset, true) as u32;
            self.extab_data.pc_actions.push(pcaction);
        }

        self.offset += 4; //Skip the terminator

        //If there are still bytes remaining, there are action entries to process
        while self.offset < self.length {
            //Console.WriteLine("Offset: " + offset);
            self.parse_action_entry()?;
        }

        Ok(())
    }

    fn parse_action_entry(&mut self) -> Result<(), ExtabDecodeError> {
        let mut exaction = ExceptionAction::new();
        exaction.action_offset = self.offset as u32;
        let action_type_byte = mem_utils::read_byte(&self.data, &mut self.offset, true);
        exaction.has_end_bit = (action_type_byte & 0x80) != 0;
        let action_type_value: u32 = (action_type_byte & 0x7F) as u32;
        let result = ExAction::from_int(action_type_value as i32);
        exaction.action_type = match result {
            Some(action) => action,
            None => {
                return Err(ExtabDecodeError::InvalidActionValue(
                    action_type_value,
                    exaction.action_offset,
                ))
            }
        };
        exaction.action_param = mem_utils::read_byte(&self.data, &mut self.offset, true);

        //Since the way action data is stored is too varied, we just store the remaining data as a byte
        //array to be used later.
        let mut size: i32;

        match exaction.action_type {
            ExAction::EndOfList => {
                size = 0;
            }
            ExAction::Branch => {
                size = 2;
            }
            ExAction::DestroyLocal => {
                size = 6;
            }
            ExAction::DestroyLocalCond => {
                size = 10;
            }
            ExAction::DestroyLocalPointer => {
                size = 6;
            }
            ExAction::DestroyLocalArray => {
                size = 10;
            }
            ExAction::DestroyBase | ExAction::DestroyMember => {
                size = 10;
            }
            ExAction::DestroyMemberCond => {
                size = 14;
            }
            ExAction::DestroyMemberArray => {
                size = 18;
            }
            ExAction::DeletePointer => {
                size = 6;
            }
            ExAction::DeletePointerCond => {
                size = 10;
            }
            ExAction::CatchBlock => {
                size = 10;
            }
            ExAction::ActiveCatchBlock => {
                size = 2;
            }
            ExAction::Terminate => {
                size = 0;
            }
            ExAction::Specification => {
                size = 10;
                //Calculate the length of the array, and add it to the base size
                let length = mem_utils::read_uint16(&self.data, &mut self.offset, false) as i32;
                size += length * 4;
            }
            ExAction::CatchBlock32 => {
                size = 14;
            }
        }

        let start_index = self.offset as usize;
        let end_index = (self.offset + size) as usize;
        exaction.bytes = self.data[start_index..end_index].into();
        self.offset += size;

        //Check if the action entry has a dtor reference. If so, get the relocation information from it,
        //and add it to the list.
        if exaction.has_dtor_ref() {
            let (offset, addr) = match exaction.get_dtor_relocation() {
                Some(val) => val,
                None => {
                    //If None was returned even though the action should have a reference, return an error
                    return Err(ExtabDecodeError::Internal);
                }
            };

            let reloc_offset: u32 = (start_index as u32) + offset;
            let reloc = Relocation { offset: reloc_offset, address: addr };
            self.extab_data.relocations.push(reloc);
        }

        self.extab_data.exception_actions.push(exaction);
        Ok(())
    }
}

/// Decodes the provided exception table data.
///
/// Returns 'None' if the table is not valid.
pub fn decode_extab(data: &[u8]) -> Result<ExceptionTableData, ExtabDecodeError> {
    let mut decoder = ExtabDecoder::new();
    decoder.parse_exception_table(data)?;
    Ok(decoder.extab_data)
}

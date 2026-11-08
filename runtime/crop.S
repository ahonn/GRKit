/* The camera's own crop factor (runtime/crop.h), for the components that
   set it or draw the frame of a crop: runtime/framing.S and runtime/menu.S
   include this file. C uses soft-float; the getter and the setter of the
   camera settings take the factor in s0. */
#ifdef FW_CROP_SET
.fpu vfpv3
.arm
/* The float bits of the factor. */
.global crop_native_bits
.type crop_native_bits, %function
crop_native_bits:
    push {r4,lr}
    ldr r12, =FW_SETTINGS
    blx r12
    ldr r12, =FW_CROP_GET
    blx r12
    vmov r0, s0
    pop {r4,pc}
/* Set the factor of float bits r0, as the native menu does. */
.global crop_native_set
.type crop_native_set, %function
crop_native_set:
    push {r4,lr}
    mov r4, r0
    ldr r12, =FW_SETTINGS
    blx r12
    vmov s0, r4
    ldr r12, =FW_CROP_SET
    blx r12
    pop {r4,pc}
/* The factor in 1/65536. */
.global crop_native_q16
.type crop_native_q16, %function
crop_native_q16:
    push {r4,lr}
    bl crop_native_bits
    vmov s0, r0
    ldr r0, =0x47800000
    vmov s1, r0
    vmul.f32 s0, s0, s1
    vcvt.u32.f32 s0, s0
    vmov r0, s0
    pop {r4,pc}
.ltorg
#endif

    .org $8000

reset:
    lda #$ff
    sta $6002

    lda #$01
    sta $6000
    jmp loop_left

ini_left:
    rol
loop_left:
    rol
    bcs ini_right
    sta $6000
    jmp loop_left

ini_right:
    ror
loop_right:
    ror
    bcs ini_left
    sta $6000
    jmp loop_right

    .blk 3, $ea

    .org $fffc
    .word reset
    .word $0000
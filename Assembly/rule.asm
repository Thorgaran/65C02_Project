VIA   = $6000
PORTB = VIA
PORTA = VIA + 1
DDRB  = VIA + 2
DDRA  = VIA + 3

RULE_ARRAY = $00        ;Addresses 0x00 to 0x07 will be used to store the rule
GEN_STATE  = $08        ;This address is used to store the generation state during computations

RULEVAL    = 110        ;Parameter of this program: the rule used (most common are 30, 90, 110, 184)
INIGEN     = %00000001  ;Parameter of this program: the initial state of the 8 bits

        .org $8000

reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        stx DDRB        ;Initialize Port B to full output

        lda #RULEVAL    ;Load the rule value
        rol             ;Rotate A left, the carry doesn't matter for this first rotate
        ldx #7          ;Load DECIMAL 7 to do 8 loop iterations
rule_ini:
        rol             ;Rotate A left
        tay             ;Save the rotated rule value in Y
        and #%00000001  ;Keep only bit 0
        sta RULE_ARRAY,X;Store this rule to RULE_ARRAY[X]
        tya             ;Get the rotated rule value back from Y
        dex             ;Decrement X
        bpl rule_ini    ;Is X still positive? Yes? Continue rule initialization

        lda #INIGEN     ;Initialize A with the first generation value
        asl             ;Shift A left

nextgen:
        ldy #15         ;Load DECIMAL 15 to do 8 loop iterations
        ror             ;Rotate A right. Carry comes from a previous operation
        sta PORTB       ;Print A
        asl             ;Shift A left
        sta GEN_STATE   ;Save new GEN_STATE (bit 0 is now reset)

nextbit:
        and #%00000111  ;Keep only the rightmost three bits
        dey             ;Decrement Y (first time)
        bne skip        ;Is it the last loop iteration?
        and #%00000011  ;Yes? Keep only the rightmost two bits
skip:   tax             ;Transfer A to X for future indexing
        lda GEN_STATE   ;Fetch once again the current gen state
        and #%11111110  ;Reset bit 0 for the next operation
        ora RULE_ARRAY,X;Set bit 0 depending on the previous gen's nearby bits and the current rule
        ror             ;Rotate A right to setup for the next bit. Carry comes from a previous operation
        sta GEN_STATE   ;Store this new gen state
        dey             ;Decrement Y (second time)
        bpl nextbit     ;Is this the last loop iteration?
        jmp nextgen     ;Yes? Go to next generation

nmi:
irq:    lda #$ff        ;Error routine (if a BRK happened)
        sta PORTB       ;Print 11111111
        stz PORTB       ;Print 00000000
        jmp irq         ;Loop error

        .blk 3, $ea     ;Mark the end of the program, used when programming the EEPROM

        .org $fffa      ;Reset vector root
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector
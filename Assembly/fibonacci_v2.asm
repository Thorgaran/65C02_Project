VIA     = $6000
PORTB   = VIA
PORTA   = VIA + 1
DDRB    = VIA + 2
DDRA    = VIA + 3

LED_RS = %10000000
LED_RW = %01000000
LED_E  = %00100000

divisor = $0000         ;4 bytes
div_val = $0004         ;4 bytes
div_mod = $0008         ;4 bytes
div_tmp = $000c         ;3 bytes
number_str = $000f      ;11 bytes
fib_cur = $001a         ;4 bytes
fib_prev = $001e        ;4 bytes
msg_ptr = $0022         ;2 bytes
step_count = $0024      ;1 byte

        .org $8000

;Prepare LCD screen when it first is powered up
lcd_init:
        lda #%00110011  ;Set 8-bit mode twice
        jsr lcd_cmd

        lda #%00110010  ;Set 8-bit mode then 4-bit mode
        jsr lcd_cmd

        lda #%00101000  ;Function set: 4-bit mode, 2 lines, 5x8 pixels
        jsr lcd_cmd

        lda #%00000001  ;Screen clear
        jsr lcd_cmd

        rts

;Send the contents of A as a command to the LCD
lcd_cmd:
        phx             ;Save X contents on stack
        tax             ;Save command into X
        
        lsr
        lsr
        lsr
        lsr             ;Shift the 4 high-order bits into the low 4 bits

        sta PORTB       ;Send first half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        txa             ;Retrieve saved command
        and #%00001111  ;Keep the 4 low-order bits
        
        sta PORTB       ;Send second half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        plx             ;Retrieve X contents from stack
        rts

;Send the contents of A as data to the LCD
lcd_data:
        phx             ;Save X contents on stack
        tax             ;Save command into X
        
        lsr
        lsr
        lsr
        lsr             ;Shift the 4 high-order bits into the low 4 bits

        ora #LED_RS
        sta PORTB       ;Send first half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        txa             ;Retrieve saved command
        and #%00001111  ;Keep the 4 low-order bits
        
        ora #LED_RS
        sta PORTB       ;Send second half of the command

        ora #LED_E
        sta PORTB
        and #~LED_E
        sta PORTB       ;Send pulse on the Enable pin of the LCD

        plx             ;Retrieve X contents from stack
        rts

;Prints the contents of number_str to the LCD
lcd_print:
        ldy #0
lcd_print_nextchar:
        lda (msg_ptr), Y;Follow the pointer to the message
        beq lcd_print_end
        jsr lcd_data    ;Print char
        
        iny             ;Set index to next char
        jmp lcd_print_nextchar

lcd_print_end:
        rts

;Divide <div_val> by A. Puts the quotient in <div_val> and the remainder in A
;Only the first byte of <div_val> is read and written to
div_8:
        sta divisor     ;Store divisor in RAM
div_8_skip_store:
        ;Compare dividend and divisor to shortcut in specific cases
        lda div_val
        cmp divisor
        bcs div_8_continue

        stz div_val
        rts

div_8_eq:
        lda #1
        sta div_val
        lda #0
        rts

div_8_continue:
        beq div_8_eq    ;Putting this here is faster than above, IDK why
        lda #0          ;Clear remainder
        clc

        ldy #8
div_8_nextbit:
        rol div_val     ;Rotate quotient
        rol             ;Rotate remainder

        tax                     ;Save old div_mod
        sec
        sbc divisor             ;Do dividend - divisor
        bcs div_8_keep_res      ;Branch if dividend < divisor

        txa                     ;Revert to old_div_mod
div_8_keep_res:
        dey
        bne div_8_nextbit

        rol div_val             ;Shift the last bit of the quotient
        rts

;Divide <div_val> by <divisor>. Puts the quotient in <div_val> and the remainder in <div_mod>
;Only the first two bytes of these addresses are read and written to
div_16:
        ;Use div_8 instead if both the dividend and divisor are smaller than 256
        lda div_val + 1
        ora divisor + 1
        bne div_16_continue

        jsr div_8_skip_store
        sta div_mod
        stz div_mod + 1
        rts

div_16_continue:
        ;Clear remainder
        stz div_mod
        stz div_mod + 1
        clc

        ldy #16
div_16_nextbit:
        ;Rotate quotient and remainder
        rol div_val
        rol div_val + 1
        rol div_mod
        rol div_mod + 1

        ;Do dividend - divisor
        sec
        lda div_mod
        sbc divisor
        tax
        lda div_mod + 1
        sbc divisor + 1
        bcc div_16_ignore_res   ;Branch if dividend < divisor

        ;Store result in div_mod
        sta div_mod + 1
        stx div_mod

div_16_ignore_res:
        dey
        bne div_16_nextbit

        rol div_val     ;Shift the last bit of the quotient
        rol div_val + 1
        rts

;Divide <div_val> by <divisor>. Puts the quotient in <div_val> and the remainder in <div_mod>
div_32:
        ;Use div_16 instead if both the dividend and divisor are smaller than 65_536
        lda div_val + 2
        ora div_val + 3
        ora divisor + 2
        ora divisor + 3
        bne div_32_continue

        jsr div_16
        stz div_mod + 2
        stz div_mod + 3
        rts

div_32_continue:
        ;Clear remainder
        stz div_mod
        stz div_mod + 1
        stz div_mod + 2
        stz div_mod + 3
        clc

        ldy #32
div_32_nextbit:
        ;Rotate quotient and remainder
        rol div_val
        rol div_val + 1
        rol div_val + 2
        rol div_val + 3
        rol div_mod
        rol div_mod + 1
        rol div_mod + 2
        rol div_mod + 3

        ;Do dividend - divisor
        sec
        lda div_mod
        sbc divisor
        sta div_tmp
        lda div_mod + 1
        sbc divisor + 1
        sta div_tmp + 1
        lda div_mod + 2
        sbc divisor + 2
        sta div_tmp + 2
        lda div_mod + 3
        sbc divisor + 3
        bcc div_32_ignore_res   ;Branch if dividend < divisor

        ;Store result in div_mod
        sta div_mod + 3
        lda div_tmp + 2
        sta div_mod + 2
        lda div_tmp + 1
        sta div_mod + 1
        lda div_tmp
        sta div_mod

div_32_ignore_res:
        dey
        bne div_32_nextbit

        rol div_val     ;Shift the last bit of the quotient
        rol div_val + 1
        rol div_val + 2
        rol div_val + 3
        rts

;Convert <div_val> to a sequence of ASCII decimal numbers put in <number_str>
;(with most significant digit first)
bin_to_dec_32:
        ;Put 10 in divisor
        lda #10
        sta divisor
        stz divisor + 1
        stz divisor + 2
        stz divisor + 3

        stz number_str          ;Store terminating \0 in string
bin_to_dec_32_nextdiv:
        jsr div_32

        lda div_mod
        clc
        adc #"0"                ;Convert decimal number to ASCII value

        ;Insert number in string
        ldx #0
insert_number_str:
        ldy number_str, X       ;Y contains next char
        sta number_str, X       ;A contains current char
        inx
        tya                     ;Next char becomes current char
        bne insert_number_str
        stz number_str, X       ;Don't forget to store terminating \0!
        
        lda div_val
        ora div_val + 1
        ora div_val + 2
        ora div_val + 3
        bne bin_to_dec_32_nextdiv;Continue dividing while the quotient isn't 0

        rts

step_str:
        .string "Step: 0"

;Main program
irq:
nmi:
reset:  ldx #$ff
        txs             ;Initialize stack pointer to address 01ff
        
        lda #%11101111  ;Set output B pins 
        sta DDRB        ;Initialize Port B to one input and full output

        lda #%11111111  ;DEBUG
        sta DDRA        ;DEBUG

        jsr lcd_init    ;Set up LCD screen
        
        lda #%00000110  ;Entry set: increment, shift cursor
        jsr lcd_cmd

        ;Print step message
        lda #<step_str
        sta msg_ptr
        lda #>step_str
        sta msg_ptr + 1
        jsr lcd_print

        ;Store number_str address in msg_ptr
        lda #<number_str
        sta msg_ptr
        lda #>number_str
        sta msg_ptr + 1

        ;Initialize fib_prev to 0
        stz fib_prev
        stz fib_prev + 1
        stz fib_prev + 2
        stz fib_prev + 3

        ;Initialize fib_cur to 1
        lda #1
        sta fib_cur
        stz fib_cur + 1
        stz fib_cur + 2
        stz fib_cur + 3

        lda #1
        sta step_count
fib_next:
        ;Convert step_count to decimal
        lda step_count
        sta div_val
        stz div_val + 1
        stz div_val + 2
        stz div_val + 3
        jsr bin_to_dec_32

        ;Print number at the right spot
        lda #$80 | $06  ;Set DDRAM address
        jsr lcd_cmd
        jsr lcd_print

        ;Compute n+1 = n + n-1
        clc

        lda fib_cur
        tax
        adc fib_prev
        stx fib_prev
        sta div_val
        sta fib_cur

        lda fib_cur + 1
        tax
        adc fib_prev + 1
        stx fib_prev + 1
        sta div_val + 1
        sta fib_cur + 1

        lda fib_cur + 2
        tax
        adc fib_prev + 2
        stx fib_prev + 2
        sta div_val + 2
        sta fib_cur + 2

        lda fib_cur + 3
        tax
        adc fib_prev + 3
        bcs prgm_end
        stx fib_prev + 3
        sta div_val + 3
        sta fib_cur + 3

        ;Convert fib_cur to decimal
        jsr bin_to_dec_32

        ;Print number at the right spot
        lda #$80 | $40  ;Set DDRAM address
        jsr lcd_cmd
        jsr lcd_print

        inc step_count
        jmp fib_next

prgm_end:
        stp

        .blk 3, $ea

        .org $fffa
        .word nmi       ;NMI vector
        .word reset     ;Reset vector
        .word irq       ;IRQ vector
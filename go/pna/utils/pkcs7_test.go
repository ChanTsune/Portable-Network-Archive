package utils

import (
	"reflect"
	"testing"
)

func Test_pkcs7unpad(t *testing.T) {
	type args struct {
		data      []byte
		blockSize int
	}
	tests := []struct {
		name    string
		args    args
		want    []byte
		wantErr bool
	}{
		{"", args{[]byte{0x01, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07}, 8}, []byte{0x01}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := pkcs7unpad(tt.args.data, tt.args.blockSize)
			if (err != nil) != tt.wantErr {
				t.Errorf("pkcs7unpad() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("pkcs7unpad() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_pkcs7pad(t *testing.T) {
	type args struct {
		data      []byte
		blockSize int
	}
	tests := []struct {
		name    string
		args    args
		want    []byte
		wantErr bool
	}{
		{"", args{[]byte{0x01}, 8}, []byte{0x01, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := pkcs7pad(tt.args.data, tt.args.blockSize)
			if (err != nil) != tt.wantErr {
				t.Errorf("pkcs7pad() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("pkcs7pad() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_pkcs7pad_unpad(t *testing.T) {
	type args struct {
		data      []byte
		blockSize int
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{"", args{[]byte{0x01}, 16}, false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := pkcs7pad(tt.args.data, tt.args.blockSize)
			if (err != nil) != tt.wantErr {
				t.Errorf("pkcs7pad() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			got2, err := pkcs7unpad(got, tt.args.blockSize)
			if (err != nil) != tt.wantErr {
				t.Errorf("pkcs7unpad() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(tt.args.data, got2) {
				t.Errorf("can not unpad... = %v, want %v", got, got2)
			}
		})
	}
}

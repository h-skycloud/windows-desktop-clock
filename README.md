# Masaüstü Saati

Windows 11 için Rust ile yazılmış, tek iş parçacıklı bir Win32 masaüstü saati. Windows’un yerel saatini 24 saat biçiminde gösterir. Saniyeler kapalıyken saati dakikada bir, açıkken saniyede bir günceller.

## Kullanım

- `MasaustuSaati.exe` dosyasını açın. Varsayılan konum ana ekranın üst ortasıdır.
- Yerini değiştirmek için saati sol tuşla sürükleyin.
- **Arka plan saydamlığı**, **Saniyeleri göster**, **Başlangıç konumu** ve **Kapat** seçenekleri için saatin üzerinde sağ tıklayın. Arka plan %100 saydamken rakamlara sağ tıklayın.
- **Son konum**, saatin sürüklenerek bırakıldığı yeri bir sonraki açılış için kaydeder. Ayarlar `%LOCALAPPDATA%\MasaustuSaati\settings.txt` dosyasına yazılır.

## Seçenekler

- **Genel** bölümünde Windows oturumu açıldığında saatin çalışmasını ve saniye gösterimini açıp kapatabilirsiniz. Windows başlangıç seçeneği geçerli kullanıcı hesabına uygulanır.
- **Görünüm** bölümünde yazı tipini, saat/dakika/saniye boyutlarını, rakam ve arka plan renklerini, saydamlığı değiştirebilirsiniz. Sağdaki önizleme değişiklikleri anında gösterir.
- **Konum** bölümünde bağlı ekranlar Windows’taki yerleşimleriyle görünür. Saat işaretini haritada sürükleyin veya ekran ve hizalama seçin. Kenara tam yaslamak için boşluğu 0 piksel yapın. Açıyı derece olarak yazın veya **45° uygula** düğmesini kullanın.
- **Kaydet** değişiklikleri masaüstündeki saate uygular. Seçenekler penceresini kapatmak ve **Vazgeç** taslağı bırakır.

Space Mono Bold yazı tipi uygulamaya gömülüdür. Yazı tipinin [SIL Open Font License 1.1](assets/Space-Mono-LICENSE.txt) metni kaynak paketinde yer alır.

## Derleme

Windows üzerinde Rust ve Microsoft C++ Build Tools ile:

```powershell
cargo build --locked --release
```

Çalıştırılabilir dosya `target/release/MasaustuSaati.exe` konumuna yazılır.

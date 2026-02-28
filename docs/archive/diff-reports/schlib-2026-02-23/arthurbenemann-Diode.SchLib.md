# arthurbenemann-Diode.SchLib

## Version
Original: Header:        Protel for Windows - Schematic Library Editor Binary File Version 5.0
Minor version: 2

## Save-As Result
Success
```
Saved: /home/kiselev/git/altium-cli-simplified/data/schlib/arthurbenemann-Diode.SchLib -> /home/kiselev/git/altium-cli-simplified/data/schlib-saveas/arthurbenemann-Diode.SchLib
```

## CFB Diff
```
Files differ: first difference at byte offset 0x0000001a (26)
  File sizes differ: 32256 bytes vs 49152 bytes
  storage OK: /1N5819
  stream DIFFERS: /1N5819/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /1N5819/Data: text(295 bytes) vs text(295 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=1N5819|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=JWHLKWLG|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=1N5819
      file2 text: |RECORD=1|LibReference=1N5819|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=JWHLKWLG|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=1N5819
    block[3] differs in /1N5819/Data: text(123 bytes) vs text(123 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LOCATION.Y=-20|CORNER.X=5|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|Location.Y=-20|Corner.X=5|Corner.Y=-20|LineWidth=1|Color=16711680
    block[4] differs in /1N5819/Data: text(126 bytes) vs text(126 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LOCATION.X=-5|LOCATION.Y=-20|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|Location.X=-5|Location.Y=-20|Corner.Y=-20|LineWidth=1|Color=16711680
    block[5] differs in /1N5819/Data: text(160 bytes) vs text(160 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
    block[6] differs in /1N5819/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=GXONAKUT
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=GXONAKUT
    block[7] differs in /1N5819/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=NPGEDPWU
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=NPGEDPWU
  stream DIFFERS: /1N5819/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /1N5819/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=2
      file2 text: |HEADER=PinPackageLength|Weight=2
    block[1] differs in /1N5819/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /1N5819/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /1N5819/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /1N5819/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /1N5819/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /1N5819/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /1N5819/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /1N5819/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=2
      file2 text: |HEADER=PinTextData|Weight=2
    block[1] differs in /1N5819/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /1N5819/PinTextData: binary(24 bytes) vs binary(24 bytes)
  storage OK: /BAS16HTWQ
  stream DIFFERS: /BAS16HTWQ/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /BAS16HTWQ/Data: text(267 bytes) vs text(267 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=BAS16HTWQ|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=UKOIXTHB|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=BAS16HTWQ
      file2 text: |RECORD=1|LibReference=BAS16HTWQ|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=UKOIXTHB|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=BAS16HTWQ
    block[1] differs in /BAS16HTWQ/Data: text(135 bytes) vs text(135 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=-50|CORNER.X=20|CORNER.Y=10|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=-10|Location.Y=-50|Corner.X=20|Corner.Y=10|Color=128|AreaColor=11599871|IsSolid=T
    block[4] differs in /BAS16HTWQ/Data: text(91 bytes) vs text(91 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10
    block[5] differs in /BAS16HTWQ/Data: text(96 bytes) vs text(96 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=10|X2=20
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=10|X2=20
    block[6] differs in /BAS16HTWQ/Data: text(145 bytes) vs text(145 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|Y1=5|X2=10|Y3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|Y1=5|X2=10|Y3=-5
    block[7] differs in /BAS16HTWQ/Data: text(124 bytes) vs text(124 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-5|CORNER.X=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|Location.X=10|Location.Y=-5|Corner.X=10|LineWidth=1|Color=16711680
    block[8] differs in /BAS16HTWQ/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=7|OWNERPARTID=1|LOCATION.X=10|CORNER.X=10|CORNER.Y=5|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=7|OwnerPartId=1|Location.X=10|Corner.X=10|Corner.Y=5|LineWidth=1|Color=16711680
    block[9] differs in /BAS16HTWQ/Data: text(105 bytes) vs text(105 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=8|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|Y1=-20|Y2=-20
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=8|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|Y1=-20|Y2=-20
    block[10] differs in /BAS16HTWQ/Data: text(110 bytes) vs text(110 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=9|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=10|Y1=-20|X2=20|Y2=-20
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=9|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=10|Y1=-20|X2=20|Y2=-20
    block[11] differs in /BAS16HTWQ/Data: text(156 bytes) vs text(156 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=10|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|Y1=-15|X2=10|Y2=-20|Y3=-25
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=10|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|Y1=-15|X2=10|Y2=-20|Y3=-25
    block[12] differs in /BAS16HTWQ/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=11|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-25|CORNER.X=10|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=11|OwnerPartId=1|Location.X=10|Location.Y=-25|Corner.X=10|Corner.Y=-20|LineWidth=1|Color=16711680
    block[13] differs in /BAS16HTWQ/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=12|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-20|CORNER.X=10|CORNER.Y=-15|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=12|OwnerPartId=1|Location.X=10|Location.Y=-20|Corner.X=10|Corner.Y=-15|LineWidth=1|Color=16711680
    block[14] differs in /BAS16HTWQ/Data: text(106 bytes) vs text(106 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=13|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=-10|Y1=-40|Y2=-40
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=13|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=-10|Y1=-40|Y2=-40
    block[15] differs in /BAS16HTWQ/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=14|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=2|X1=10|Y1=-40|X2=20|Y2=-40
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=14|OwnerPartId=1|LineWidth=1|LocationCount=2|X1=10|Y1=-40|X2=20|Y2=-40
    block[16] differs in /BAS16HTWQ/Data: text(156 bytes) vs text(156 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=15|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|Y1=-35|X2=10|Y2=-40|Y3=-45
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=15|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|Y1=-35|X2=10|Y2=-40|Y3=-45
    block[17] differs in /BAS16HTWQ/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=16|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-45|CORNER.X=10|CORNER.Y=-40|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=16|OwnerPartId=1|Location.X=10|Location.Y=-45|Corner.X=10|Corner.Y=-40|LineWidth=1|Color=16711680
    block[18] differs in /BAS16HTWQ/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=17|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-40|CORNER.X=10|CORNER.Y=-35|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=17|OwnerPartId=1|Location.X=10|Location.Y=-40|Corner.X=10|Corner.Y=-35|LineWidth=1|Color=16711680
    block[23] differs in /BAS16HTWQ/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=WNKAWXDP
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=WNKAWXDP
    block[24] differs in /BAS16HTWQ/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=VUUYUDBV
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=VUUYUDBV
  stream DIFFERS: /BAS16HTWQ/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 36 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 36 00 
    block[0] differs in /BAS16HTWQ/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=6
      file2 text: |HEADER=PinPackageLength|Weight=6
    block[1] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[3] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[4] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[5] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[6] differs in /BAS16HTWQ/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /BAS16HTWQ/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 36 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 36 00 
    block[0] differs in /BAS16HTWQ/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=6
      file2 text: |HEADER=PinSymbolLineWidth|Weight=6
    block[1] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[5] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[6] differs in /BAS16HTWQ/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  storage OK: /BAT54C
  stream DIFFERS: /BAT54C/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /BAT54C/Data: text(295 bytes) vs text(295 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=BAT54C|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=FWEDLHFE|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=BAT54C
      file2 text: |RECORD=1|LibReference=BAT54C|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=FWEDLHFE|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=BAT54C
    block[4] differs in /BAT54C/Data: text(114 bytes) vs text(114 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=4|X1=40|X2=20|X3=30|X4=30|Y4=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|LineWidth=1|LocationCount=4|X1=40|X2=20|X3=30|X4=30|Y4=10
    block[5] differs in /BAT54C/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LOCATION.X=20|CORNER.X=20|CORNER.Y=5|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|Location.X=20|Corner.X=20|Corner.Y=5|LineWidth=1|Color=16711680
    block[6] differs in /BAT54C/Data: text(124 bytes) vs text(124 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LOCATION.X=20|LOCATION.Y=-5|CORNER.X=20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|Location.X=20|Location.Y=-5|Corner.X=20|LineWidth=1|Color=16711680
    block[7] differs in /BAT54C/Data: text(157 bytes) vs text(157 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=10|Y1=5|X2=20|X3=10|Y3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=10|Y1=5|X2=20|X3=10|Y3=-5
    block[8] differs in /BAT54C/Data: text(122 bytes) vs text(122 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=7|OWNERPARTID=1|LOCATION.X=40|CORNER.X=40|CORNER.Y=-5|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=7|OwnerPartId=1|Location.X=40|Corner.X=40|Corner.Y=-5|LineWidth=1|Color=16711680
    block[9] differs in /BAT54C/Data: text(123 bytes) vs text(123 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=8|OWNERPARTID=1|LOCATION.X=40|LOCATION.Y=5|CORNER.X=40|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=8|OwnerPartId=1|Location.X=40|Location.Y=5|Corner.X=40|LineWidth=1|Color=16711680
    block[10] differs in /BAT54C/Data: text(157 bytes) vs text(157 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=9|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=50|Y1=-5|X2=40|X3=50|Y3=5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=9|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=50|Y1=-5|X2=40|X3=50|Y3=5
    block[11] differs in /BAT54C/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=HMWREMFB
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=HMWREMFB
    block[12] differs in /BAT54C/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=OOGCWDBK
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=OOGCWDBK
  stream DIFFERS: /BAT54C/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54C/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=3
      file2 text: |HEADER=PinPackageLength|Weight=3
    block[1] differs in /BAT54C/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /BAT54C/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[3] differs in /BAT54C/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /BAT54C/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54C/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=3
      file2 text: |HEADER=PinSymbolLineWidth|Weight=3
    block[1] differs in /BAT54C/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /BAT54C/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /BAT54C/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /BAT54C/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54C/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=3
      file2 text: |HEADER=PinTextData|Weight=3
    block[1] differs in /BAT54C/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /BAT54C/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[3] differs in /BAT54C/PinTextData: binary(24 bytes) vs binary(24 bytes)
  storage OK: /BAT54SFILM
  stream DIFFERS: /BAT54SFILM/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /BAT54SFILM/Data: text(303 bytes) vs text(303 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=BAT54SFILM|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=DIFGAYHP|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=BAT54SFILM
      file2 text: |RECORD=1|LibReference=BAT54SFILM|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=DIFGAYHP|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=BAT54SFILM
    block[4] differs in /BAT54SFILM/Data: text(114 bytes) vs text(114 bytes)
      file1 text: |RECORD=6|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LINEWIDTH=1|LOCATIONCOUNT=4|X1=40|X2=20|X3=30|X4=30|Y4=10
      file2 text: |RECORD=6|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|LineWidth=1|LocationCount=4|X1=40|X2=20|X3=30|X4=30|Y4=10
    block[5] differs in /BAT54SFILM/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LOCATION.X=20|CORNER.X=20|CORNER.Y=5|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|Location.X=20|Corner.X=20|Corner.Y=5|LineWidth=1|Color=16711680
    block[6] differs in /BAT54SFILM/Data: text(124 bytes) vs text(124 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LOCATION.X=20|LOCATION.Y=-5|CORNER.X=20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|Location.X=20|Location.Y=-5|Corner.X=20|LineWidth=1|Color=16711680
    block[7] differs in /BAT54SFILM/Data: text(157 bytes) vs text(157 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=10|Y1=5|X2=20|X3=10|Y3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=10|Y1=5|X2=20|X3=10|Y3=-5
    block[8] differs in /BAT54SFILM/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=7|OWNERPARTID=1|LOCATION.X=50|CORNER.X=50|CORNER.Y=5|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=7|OwnerPartId=1|Location.X=50|Corner.X=50|Corner.Y=5|LineWidth=1|Color=16711680
    block[9] differs in /BAT54SFILM/Data: text(124 bytes) vs text(124 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=8|OWNERPARTID=1|LOCATION.X=50|LOCATION.Y=-5|CORNER.X=50|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=8|OwnerPartId=1|Location.X=50|Location.Y=-5|Corner.X=50|LineWidth=1|Color=16711680
    block[10] differs in /BAT54SFILM/Data: text(157 bytes) vs text(157 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=9|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=40|Y1=5|X2=50|X3=40|Y3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=9|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=40|Y1=5|X2=50|X3=40|Y3=-5
    block[11] differs in /BAT54SFILM/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=NEEOUVCC
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=NEEOUVCC
    block[12] differs in /BAT54SFILM/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=VOBXWKFD
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=VOBXWKFD
  stream DIFFERS: /BAT54SFILM/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54SFILM/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=3
      file2 text: |HEADER=PinPackageLength|Weight=3
    block[1] differs in /BAT54SFILM/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /BAT54SFILM/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[3] differs in /BAT54SFILM/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /BAT54SFILM/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54SFILM/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=3
      file2 text: |HEADER=PinSymbolLineWidth|Weight=3
    block[1] differs in /BAT54SFILM/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /BAT54SFILM/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /BAT54SFILM/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /BAT54SFILM/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 33 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 33 00 
    block[0] differs in /BAT54SFILM/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=3
      file2 text: |HEADER=PinTextData|Weight=3
    block[1] differs in /BAT54SFILM/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /BAT54SFILM/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[3] differs in /BAT54SFILM/PinTextData: binary(24 bytes) vs binary(24 bytes)
  storage OK: /Burns-SOD323-T08C
  stream DIFFERS: /Burns-SOD323-T08C/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Burns-SOD323-T08C/Data: text(317 bytes) vs text(317 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Burns-SOD323-T08C|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=PTPPXQMS|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Burns-SOD323-T08C
      file2 text: |RECORD=1|LibReference=Burns-SOD323-T08C|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=PTPPXQMS|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Burns-SOD323-T08C
    block[1] differs in /Burns-SOD323-T08C/Data: text(110 bytes) vs text(110 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=-10|CORNER.X=-10|CORNER.Y=-10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|OwnerPartId=1|Location.X=-10|Corner.X=-10|Corner.Y=-10|LineWidth=1|Color=16711680
    block[2] differs in /Burns-SOD323-T08C/Data: text(140 bytes) vs text(140 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=1|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=-20|CORNER.X=-10|CORNER.Y=-30|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=1|OwnerPartId=1|Location.X=-10|Location.Y=-20|Corner.X=-10|Corner.Y=-30|LineWidth=1|Color=16711680
    block[3] differs in /Burns-SOD323-T08C/Data: text(138 bytes) vs text(138 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=20|CORNER.X=-10|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|Location.X=-10|Location.Y=20|Corner.X=-10|Corner.Y=10|LineWidth=1|Color=16711680
    block[4] differs in /Burns-SOD323-T08C/Data: text(125 bytes) vs text(125 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-10|CORNER.X=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|Location.X=10|Location.Y=-10|Corner.X=10|LineWidth=1|Color=16711680
    block[5] differs in /Burns-SOD323-T08C/Data: text(136 bytes) vs text(136 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=10|CORNER.X=10|CORNER.Y=20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|Location.X=10|Location.Y=10|Corner.X=10|Corner.Y=20|LineWidth=1|Color=16711680
    block[6] differs in /Burns-SOD323-T08C/Data: text(138 bytes) vs text(138 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-30|CORNER.X=10|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|Location.X=10|Location.Y=-30|Corner.X=10|Corner.Y=-20|LineWidth=1|Color=16711680
    block[7] differs in /Burns-SOD323-T08C/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-30|CORNER.X=-10|CORNER.Y=-30|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|Location.X=10|Location.Y=-30|Corner.X=-10|Corner.Y=-30|LineWidth=1|Color=16711680
    block[8] differs in /Burns-SOD323-T08C/Data: text(137 bytes) vs text(137 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=7|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=20|CORNER.X=-10|CORNER.Y=20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=7|OwnerPartId=1|Location.X=10|Location.Y=20|Corner.X=-10|Corner.Y=20|LineWidth=1|Color=16711680
    block[11] differs in /Burns-SOD323-T08C/Data: text(140 bytes) vs text(140 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=10|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=-20|CORNER.X=-5|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=10|OwnerPartId=1|Location.X=-10|Location.Y=-20|Corner.X=-5|Corner.Y=-20|LineWidth=1|Color=16711680
    block[12] differs in /Burns-SOD323-T08C/Data: text(141 bytes) vs text(141 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=11|OWNERPARTID=1|LOCATION.X=-15|LOCATION.Y=-20|CORNER.X=-10|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=11|OwnerPartId=1|Location.X=-15|Location.Y=-20|Corner.X=-10|Corner.Y=-20|LineWidth=1|Color=16711680
    block[13] differs in /Burns-SOD323-T08C/Data: text(170 bytes) vs text(170 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=12|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=-5|Y1=-10|X2=-10|Y2=-20|X3=-15|Y3=-10
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=12|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=-5|Y1=-10|X2=-10|Y2=-20|X3=-15|Y3=-10
    block[14] differs in /Burns-SOD323-T08C/Data: text(136 bytes) vs text(136 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=13|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=10|CORNER.X=5|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=13|OwnerPartId=1|Location.X=10|Location.Y=10|Corner.X=5|Corner.Y=10|LineWidth=1|Color=16711680
    block[15] differs in /Burns-SOD323-T08C/Data: text(137 bytes) vs text(137 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=14|OWNERPARTID=1|LOCATION.X=15|LOCATION.Y=10|CORNER.X=10|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=14|OwnerPartId=1|Location.X=15|Location.Y=10|Corner.X=10|Corner.Y=10|LineWidth=1|Color=16711680
    block[16] differs in /Burns-SOD323-T08C/Data: text(152 bytes) vs text(152 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=15|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=5|X2=10|Y2=10|X3=15
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=15|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=5|X2=10|Y2=10|X3=15
    block[17] differs in /Burns-SOD323-T08C/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=16|OWNERPARTID=1|LOCATION.X=10|LOCATION.Y=-20|CORNER.X=15|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=16|OwnerPartId=1|Location.X=10|Location.Y=-20|Corner.X=15|Corner.Y=-20|LineWidth=1|Color=16711680
    block[18] differs in /Burns-SOD323-T08C/Data: text(138 bytes) vs text(138 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=17|OWNERPARTID=1|LOCATION.X=5|LOCATION.Y=-20|CORNER.X=10|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=17|OwnerPartId=1|Location.X=5|Location.Y=-20|Corner.X=10|Corner.Y=-20|LineWidth=1|Color=16711680
    block[19] differs in /Burns-SOD323-T08C/Data: text(167 bytes) vs text(167 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=18|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=15|Y1=-10|X2=10|Y2=-20|X3=5|Y3=-10
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=18|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=15|Y1=-10|X2=10|Y2=-20|X3=5|Y3=-10
    block[20] differs in /Burns-SOD323-T08C/Data: text(137 bytes) vs text(137 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=19|OWNERPARTID=1|LOCATION.X=5|LOCATION.Y=-20|CORNER.X=3|CORNER.Y=-18|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=19|OwnerPartId=1|Location.X=5|Location.Y=-20|Corner.X=3|Corner.Y=-18|LineWidth=1|Color=16711680
    block[21] differs in /Burns-SOD323-T08C/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=20|OWNERPARTID=1|LOCATION.X=-10|LOCATION.Y=10|CORNER.X=-15|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=20|OwnerPartId=1|Location.X=-10|Location.Y=10|Corner.X=-15|Corner.Y=10|LineWidth=1|Color=16711680
    block[22] differs in /Burns-SOD323-T08C/Data: text(138 bytes) vs text(138 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=21|OWNERPARTID=1|LOCATION.X=-5|LOCATION.Y=10|CORNER.X=-10|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=21|OwnerPartId=1|Location.X=-5|Location.Y=10|Corner.X=-10|Corner.Y=10|LineWidth=1|Color=16711680
    block[23] differs in /Burns-SOD323-T08C/Data: text(155 bytes) vs text(155 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=22|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=-15|X2=-10|Y2=10|X3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=22|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=-15|X2=-10|Y2=10|X3=-5
    block[24] differs in /Burns-SOD323-T08C/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=23|OWNERPARTID=1|LOCATION.X=17|LOCATION.Y=-22|CORNER.X=15|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=23|OwnerPartId=1|Location.X=17|Location.Y=-22|Corner.X=15|Corner.Y=-20|LineWidth=1|Color=16711680
    block[25] differs in /Burns-SOD323-T08C/Data: text(136 bytes) vs text(136 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=24|OWNERPARTID=1|LOCATION.X=-3|LOCATION.Y=8|CORNER.X=-5|CORNER.Y=10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=24|OwnerPartId=1|Location.X=-3|Location.Y=8|Corner.X=-5|Corner.Y=10|LineWidth=1|Color=16711680
    block[26] differs in /Burns-SOD323-T08C/Data: text(139 bytes) vs text(139 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=25|OWNERPARTID=1|LOCATION.X=-15|LOCATION.Y=10|CORNER.X=-17|CORNER.Y=12|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=25|OwnerPartId=1|Location.X=-15|Location.Y=10|Corner.X=-17|Corner.Y=12|LineWidth=1|Color=16711680
    block[27] differs in /Burns-SOD323-T08C/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=RGFNHGIC
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=RGFNHGIC
    block[28] differs in /Burns-SOD323-T08C/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=DHSGRRWL
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=DHSGRRWL
  stream DIFFERS: /Burns-SOD323-T08C/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Burns-SOD323-T08C/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=2
      file2 text: |HEADER=PinPackageLength|Weight=2
    block[1] differs in /Burns-SOD323-T08C/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /Burns-SOD323-T08C/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /Burns-SOD323-T08C/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Burns-SOD323-T08C/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /Burns-SOD323-T08C/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /Burns-SOD323-T08C/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /Burns-SOD323-T08C/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Burns-SOD323-T08C/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=2
      file2 text: |HEADER=PinTextData|Weight=2
    block[1] differs in /Burns-SOD323-T08C/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /Burns-SOD323-T08C/PinTextData: binary(24 bytes) vs binary(24 bytes)
  storage OK: /CD2320
  stream DIFFERS: /CD2320/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /CD2320/Data: text(295 bytes) vs text(295 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=CD2320|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=OGEIKOCT|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=CD2320
      file2 text: |RECORD=1|LibReference=CD2320|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=OGEIKOCT|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=CD2320
    block[1] differs in /CD2320/Data: text(108 bytes) vs text(108 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.Y=-40|CORNER.X=40|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.Y=-40|Corner.X=40|Color=128|AreaColor=11599871|IsSolid=T
    block[6] differs in /CD2320/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=QASDRRDR
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=QASDRRDR
    block[7] differs in /CD2320/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=YIRNJCFT
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=YIRNJCFT
  stream DIFFERS: /CD2320/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 34 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 34 00 
    block[0] differs in /CD2320/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=4
      file2 text: |HEADER=PinPackageLength|Weight=4
    block[1] differs in /CD2320/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /CD2320/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[3] differs in /CD2320/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[4] differs in /CD2320/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /CD2320/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 34 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 34 00 
    block[0] differs in /CD2320/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=4
      file2 text: |HEADER=PinSymbolLineWidth|Weight=4
    block[1] differs in /CD2320/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /CD2320/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[3] differs in /CD2320/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[4] differs in /CD2320/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /CD2320/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 34 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 34 00 
    block[0] differs in /CD2320/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=4
      file2 text: |HEADER=PinTextData|Weight=4
    block[1] differs in /CD2320/PinTextData: binary(19 bytes) vs binary(19 bytes)
    block[2] differs in /CD2320/PinTextData: binary(19 bytes) vs binary(19 bytes)
    block[3] differs in /CD2320/PinTextData: binary(19 bytes) vs binary(19 bytes)
    block[4] differs in /CD2320/PinTextData: binary(19 bytes) vs binary(19 bytes)
  storage OK: /DIODE
  stream DIFFERS: /DIODE/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /DIODE/Data: text(293 bytes) vs text(293 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=DIODE|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=DBWDLSTC|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=DIODE
      file2 text: |RECORD=1|LibReference=DIODE|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=DBWDLSTC|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=DIODE
    block[1] differs in /DIODE/Data: text(108 bytes) vs text(108 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.Y=-20|CORNER.X=5|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|OwnerPartId=1|Location.Y=-20|Corner.X=5|Corner.Y=-20|LineWidth=1|Color=16711680
    block[2] differs in /DIODE/Data: text(126 bytes) vs text(126 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=1|OWNERPARTID=1|LOCATION.X=-5|LOCATION.Y=-20|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=1|OwnerPartId=1|Location.X=-5|Location.Y=-20|Corner.Y=-20|LineWidth=1|Color=16711680
    block[5] differs in /DIODE/Data: text(160 bytes) vs text(160 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
    block[6] differs in /DIODE/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=DQPWJQAQ
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=DQPWJQAQ
    block[7] differs in /DIODE/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=IKRBWIEV
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=IKRBWIEV
  stream DIFFERS: /DIODE/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /DIODE/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=2
      file2 text: |HEADER=PinPackageLength|Weight=2
    block[1] differs in /DIODE/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /DIODE/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /DIODE/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /DIODE/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /DIODE/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /DIODE/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /DIODE/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /DIODE/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=2
      file2 text: |HEADER=PinTextData|Weight=2
    block[1] differs in /DIODE/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /DIODE/PinTextData: binary(24 bytes) vs binary(24 bytes)
  storage OK: /Diode-SMA
  stream DIFFERS: /Diode-SMA/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /Diode-SMA/Data: text(301 bytes) vs text(301 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=Diode-SMA|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=JVJJEDCK|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=Diode-SMA
      file2 text: |RECORD=1|LibReference=Diode-SMA|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=JVJJEDCK|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=Diode-SMA
    block[3] differs in /Diode-SMA/Data: text(123 bytes) vs text(123 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LOCATION.Y=-20|CORNER.X=5|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|Location.Y=-20|Corner.X=5|Corner.Y=-20|LineWidth=1|Color=16711680
    block[4] differs in /Diode-SMA/Data: text(126 bytes) vs text(126 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LOCATION.X=-5|LOCATION.Y=-20|CORNER.Y=-20|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|Location.X=-5|Location.Y=-20|Corner.Y=-20|LineWidth=1|Color=16711680
    block[5] differs in /Diode-SMA/Data: text(160 bytes) vs text(160 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=5|Y1=-10|Y2=-20|X3=-5|Y3=-10
    block[6] differs in /Diode-SMA/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=FXYNUGMG
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=FXYNUGMG
    block[7] differs in /Diode-SMA/Data: text(143 bytes) vs text(143 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=*|NAME=Comment|UNIQUEID=QYRFKXNQ
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|IsHidden=T|Text=*|Name=Comment|UniqueID=QYRFKXNQ
  stream DIFFERS: /Diode-SMA/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Diode-SMA/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=2
      file2 text: |HEADER=PinPackageLength|Weight=2
    block[1] differs in /Diode-SMA/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /Diode-SMA/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /Diode-SMA/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Diode-SMA/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /Diode-SMA/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /Diode-SMA/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /Diode-SMA/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /Diode-SMA/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=2
      file2 text: |HEADER=PinTextData|Weight=2
    block[1] differs in /Diode-SMA/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /Diode-SMA/PinTextData: binary(24 bytes) vs binary(24 bytes)
  stream DIFFERS: /FileHeader
    first diff at stream offset 0x00000053 (83)
    file1 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 45 49 47 48 54 3d 31 33 
    file2 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 65 69 67 68 74 3d 31 33 
    block[0] differs in /FileHeader: text(994 bytes) vs text(994 bytes)
      file1 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|WEIGHT=132|MINORVERSION=2|UNIQUEID=IAIWUWGU|FONTIDCOUNT=4|SIZE1=10|FONTNAME1=Times New Roman|SIZE2=8|FONTNAME2=Times New Roman|SIZE3=12|BOLD3=T|FONTNAME3=Times New Roman|SIZE4=5|BOLD4=T|FONTNAME4=Times New Roman|USEMBCS=T|ISBOC=T|SHEETSTYLE=9|BORDERON=T|SHEETNUMBERSPACESIZE=4|AREACOLOR=16317695|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=2000|CUSTOMY=2000|USECUSTOMSHEET=T|REFERENCEZONESON=T|DISPLAY_UNIT=4|COMPCOUNT=9|LIBREF0=Burns-SOD323-T08C|COMPDESCR0==description|PARTCOUNT0=2|LIBREF1=BAT54SFILM|COMPDESCR1==description|PARTCOUNT1=2|LIBREF2=ZENER-SMB|COMPDESCR2==description|PARTCOUNT2=2|LIBREF3=BAS16HTWQ|PARTCOUNT3=2|LIBREF4=CD2320|COMPDESCR4==description|PARTCOUNT4=2|LIBREF5=BAT54C|COMPDESCR5==description|PARTCOUNT5=2|LIBREF6=Diode-SMA|COMPDESCR6==description|PARTCOUNT6=2|LIBREF7=1N5819|COMPDESCR7==description|PARTCOUNT7=2|LIBREF8=DIODE|COMPDESCR8==description|PARTCOUNT8=2
      file2 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|Weight=132|MinorVersion=2|UniqueID=IAIWUWGU|FontIdCount=4|Size1=10|FontName1=Times New Roman|Size2=8|FontName2=Times New Roman|Size3=12|Bold3=T|FontName3=Times New Roman|Size4=5|Bold4=T|FontName4=Times New Roman|UseMBCS=T|IsBOC=T|SheetStyle=9|BorderOn=T|SheetNumberSpaceSize=4|AreaColor=16317695|SnapGridOn=T|SnapGridSize=10|VisibleGridOn=T|VisibleGridSize=10|CustomX=2000|CustomY=2000|UseCustomSheet=T|ReferenceZonesOn=T|Display_Unit=4|CompCount=9|LibRef0=Burns-SOD323-T08C|CompDescr0==description|PartCount0=2|LibRef1=BAT54SFILM|CompDescr1==description|PartCount1=2|LibRef2=ZENER-SMB|CompDescr2==description|PartCount2=2|LibRef3=BAS16HTWQ|PartCount3=2|LibRef4=CD2320|CompDescr4==description|PartCount4=2|LibRef5=BAT54C|CompDescr5==description|PartCount5=2|LibRef6=Diode-SMA|CompDescr6==description|PartCount6=2|LibRef7=1N5819|CompDescr7==description|PartCount7=2|LibRef8=DIODE|CompDescr8==description|PartCount8=2
  stream OK: /Storage (25 bytes)
  storage OK: /ZENER-SMB
  stream DIFFERS: /ZENER-SMB/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /ZENER-SMB/Data: text(281 bytes) vs text(281 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=ZENER-SMB|COMPONENTDESCRIPTION==description|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|TARGETFILENAME=*|UNIQUEID=AUBXWUKC|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|DESIGNITEMID=ZENER-SMB
      file2 text: |RECORD=1|LibReference=ZENER-SMB|ComponentDescription==description|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|TargetFileName=*|UniqueID=AUBXWUKC|AreaColor=11599871|Color=128|PartIDLocked=T|DesignItemId=ZENER-SMB
    block[3] differs in /ZENER-SMB/Data: text(112 bytes) vs text(112 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=2|OWNERPARTID=1|LOCATION.Y=-10|CORNER.Y=-15|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=2|OwnerPartId=1|Location.Y=-10|Corner.Y=-15|LineWidth=1|Color=16711680
    block[4] differs in /ZENER-SMB/Data: text(111 bytes) vs text(111 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LOCATION.Y=-5|CORNER.Y=-10|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|Location.Y=-5|Corner.Y=-10|LineWidth=1|Color=16711680
    block[5] differs in /ZENER-SMB/Data: text(160 bytes) vs text(160 bytes)
      file1 text: |RECORD=7|ISNOTACCESIBLE=T|INDEXINSHEET=4|OWNERPARTID=1|LINEWIDTH=1|COLOR=16711680|AREACOLOR=16711680|ISSOLID=T|LOCATIONCOUNT=3|X1=10|Y1=-15|Y2=-10|X3=10|Y3=-5
      file2 text: |RECORD=7|IsNotAccesible=T|IndexInSheet=4|OwnerPartId=1|LineWidth=1|Color=16711680|AreaColor=16711680|IsSolid=T|LocationCount=3|X1=10|Y1=-15|Y2=-10|X3=10|Y3=-5
    block[6] differs in /ZENER-SMB/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=5|OWNERPARTID=1|LOCATION.Y=-5|CORNER.X=2|CORNER.Y=-3|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=5|OwnerPartId=1|Location.Y=-5|Corner.X=2|Corner.Y=-3|LineWidth=1|Color=16711680
    block[7] differs in /ZENER-SMB/Data: text(126 bytes) vs text(126 bytes)
      file1 text: |RECORD=13|ISNOTACCESIBLE=T|INDEXINSHEET=6|OWNERPARTID=1|LOCATION.X=-2|LOCATION.Y=-17|CORNER.Y=-15|LINEWIDTH=1|COLOR=16711680
      file2 text: |RECORD=13|IsNotAccesible=T|IndexInSheet=6|OwnerPartId=1|Location.X=-2|Location.Y=-17|Corner.Y=-15|LineWidth=1|Color=16711680
    block[8] differs in /ZENER-SMB/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=D?|NAME=Designator|READONLYSTATE=1|UNIQUEID=LADRKIGV
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=D?|Name=Designator|ReadOnlyState=1|UniqueID=LADRKIGV
    block[9] differs in /ZENER-SMB/Data: text(125 bytes) vs text(125 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|NAME=Comment|UNIQUEID=COECXHMD
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Name=Comment|UniqueID=COECXHMD
  stream DIFFERS: /ZENER-SMB/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /ZENER-SMB/PinPackageLength: text(34 bytes) vs text(34 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=2
      file2 text: |HEADER=PinPackageLength|Weight=2
    block[1] differs in /ZENER-SMB/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
    block[2] differs in /ZENER-SMB/PinPackageLength: binary(51 bytes) vs binary(51 bytes)
  stream DIFFERS: /ZENER-SMB/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /ZENER-SMB/PinSymbolLineWidth: text(36 bytes) vs text(36 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=2
      file2 text: |HEADER=PinSymbolLineWidth|Weight=2
    block[1] differs in /ZENER-SMB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
    block[2] differs in /ZENER-SMB/PinSymbolLineWidth: binary(54 bytes) vs binary(54 bytes)
  stream DIFFERS: /ZENER-SMB/PinTextData
    first diff at stream offset 0x00000019 (25)
    file1 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 45 49 47 48 54 3d 32 00 
    file2 hex [0x00000011+]: 78 74 44 61 74 61 7c 57 65 69 67 68 74 3d 32 00 
    block[0] differs in /ZENER-SMB/PinTextData: text(29 bytes) vs text(29 bytes)
      file1 text: |HEADER=PinTextData|WEIGHT=2
      file2 text: |HEADER=PinTextData|Weight=2
    block[1] differs in /ZENER-SMB/PinTextData: binary(24 bytes) vs binary(24 bytes)
    block[2] differs in /ZENER-SMB/PinTextData: binary(24 bytes) vs binary(24 bytes)
```

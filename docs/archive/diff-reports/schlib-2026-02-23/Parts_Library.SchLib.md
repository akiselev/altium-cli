# Parts_Library.SchLib

## Version
Original: Header:        Protel for Windows - Schematic Library Editor Binary File Version 5.0
Minor version: 2

## Save-As Result
Saved: /home/kiselev/git/altium-cli-simplified/data/schlib/Parts_Library.SchLib -> /home/kiselev/git/altium-cli-simplified/data/schlib-saveas/Parts_Library.SchLib

## CFB Diff
```
Files differ: first difference at byte offset 0x0000001a (26)
  File sizes differ: 35840 bytes vs 53248 bytes
  storage OK: /ATX_POWER_CONN
  stream DIFFERS: /ATX_POWER_CONN/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /ATX_POWER_CONN/Data: text(264 bytes) vs text(264 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=ATX_POWER_CONN|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=PXSWHUSO|AREACOLOR=11599871|COLOR=128|PARTIDLOCKED=T|ALLPINCOUNT=24
      file2 text: |RECORD=1|LibReference=ATX_POWER_CONN|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=PXSWHUSO|AreaColor=11599871|Color=128|PartIDLocked=T|AllPinCount=24
    block[1] differs in /ATX_POWER_CONN/Data: text(136 bytes) vs text(136 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|OWNERPARTID=1|LOCATION.X=90|LOCATION.Y=180|CORNER.X=220|CORNER.Y=430|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|OwnerPartId=1|Location.X=90|Location.Y=180|Corner.X=220|Corner.Y=430|Color=128|AreaColor=11599871|IsSolid=T
    block[26] differs in /ATX_POWER_CONN/Data: text(149 bytes) vs text(149 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=*|NAME=Designator|READONLYSTATE=1|UNIQUEID=VPGSJIST
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=*|Name=Designator|ReadOnlyState=1|UniqueID=VPGSJIST
    block[27] differs in /ATX_POWER_CONN/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=KKXYILSL
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=KKXYILSL
    block[29] differs in /ATX_POWER_CONN/Data: text(191 bytes) vs text(191 bytes)
      file1 text: |RECORD=45|OWNERINDEX=28|INDEXINSHEET=-1|MODELNAME=TE_1-1775099-3|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=TE_1-1775099-3|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=BWLYEKEW
      file2 text: |RECORD=45|OwnerIndex=28|IndexInSheet=-1|ModelName=TE_1-1775099-3|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=TE_1-1775099-3|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=BWLYEKEW
    block[30] differs in /ATX_POWER_CONN/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=46|OWNERINDEX=29
      file2 text: |RECORD=46|OwnerIndex=29
    block[31] differs in /ATX_POWER_CONN/Data: text(25 bytes) vs text(25 bytes)
      file1 text: |RECORD=48|OWNERINDEX=29
      file2 text: |RECORD=48|OwnerIndex=29
  stream DIFFERS: /ATX_POWER_CONN/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 32 34 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 32 34 
    block[0] differs in /ATX_POWER_CONN/PinPackageLength: text(35 bytes) vs text(35 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=24
      file2 text: |HEADER=PinPackageLength|Weight=24
  stream DIFFERS: /ATX_POWER_CONN/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 32 34 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 32 34 
    block[0] differs in /ATX_POWER_CONN/PinSymbolLineWidth: text(37 bytes) vs text(37 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=24
      file2 text: |HEADER=PinSymbolLineWidth|Weight=24
  stream DIFFERS: /FileHeader
    first diff at stream offset 0x00000053 (83)
    file1 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 45 49 47 48 54 3d 31 38 
    file2 hex [0x0000004b+]: 6f 6e 20 35 2e 30 7c 57 65 69 67 68 74 3d 31 38 
    block[0] differs in /FileHeader: text(512 bytes) vs text(512 bytes)
      file1 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|WEIGHT=188|MINORVERSION=2|UNIQUEID=TDBHBRCA|FONTIDCOUNT=1|SIZE1=10|FONTNAME1=Times New Roman|USEMBCS=T|ISBOC=T|SHEETSTYLE=9|BORDERON=T|SHEETNUMBERSPACESIZE=4|AREACOLOR=16317695|SNAPGRIDON=T|SNAPGRIDSIZE=10|VISIBLEGRIDON=T|VISIBLEGRIDSIZE=10|CUSTOMX=2000|CUSTOMY=2000|USECUSTOMSHEET=T|REFERENCEZONESON=T|DISPLAY_UNIT=4|COMPCOUNT=2|LIBREF0=MT41K256M16TW-107:P|COMPDESCR0=4Gb DDR3L SDRAM|PARTCOUNT0=2|LIBREF1=ATX_POWER_CONN|PARTCOUNT1=2
      file2 text: |HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0|Weight=188|MinorVersion=2|UniqueID=TDBHBRCA|FontIdCount=1|Size1=10|FontName1=Times New Roman|UseMBCS=T|IsBOC=T|SheetStyle=9|BorderOn=T|SheetNumberSpaceSize=4|AreaColor=16317695|SnapGridOn=T|SnapGridSize=10|VisibleGridOn=T|VisibleGridSize=10|CustomX=2000|CustomY=2000|UseCustomSheet=T|ReferenceZonesOn=T|Display_Unit=4|CompCount=2|LibRef0=MT41K256M16TW-107:P|CompDescr0=4Gb DDR3L SDRAM|PartCount0=2|LibRef1=ATX_POWER_CONN|PartCount1=2
  storage OK: /MT41K256M16TW-107_P
  stream DIFFERS: /MT41K256M16TW-107_P/Data
    first diff at stream offset 0x0000000f (15)
    file1 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 49 42 52 45 46 45 52 45 
    file2 hex [0x00000007+]: 43 4f 52 44 3d 31 7c 4c 69 62 52 65 66 65 72 65 
    block[0] differs in /MT41K256M16TW-107_P/Data: text(355 bytes) vs text(355 bytes)
      file1 text: |RECORD=1|LIBREFERENCE=MT41K256M16TW-107:P|COMPONENTDESCRIPTION=4Gb DDR3L SDRAM|PARTCOUNT=2|DISPLAYMODECOUNT=1|INDEXINSHEET=-1|OWNERPARTID=-1|CURRENTPARTID=1|LIBRARYPATH=*|SOURCELIBRARYNAME=*|SHEETPARTFILENAME=*|TARGETFILENAME=*|UNIQUEID=XEALYKYN|AREACOLOR=11599871|COLOR=128|OVERIDECOLORS=T|PARTIDLOCKED=T|DESIGNITEMID=MT41K256M16TW-107:P|ALLPINCOUNT=57
      file2 text: |RECORD=1|LibReference=MT41K256M16TW-107:P|ComponentDescription=4Gb DDR3L SDRAM|PartCount=2|DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|LibraryPath=*|SourceLibraryName=*|SheetPartFileName=*|TargetFileName=*|UniqueID=XEALYKYN|AreaColor=11599871|Color=128|OverideColors=T|PartIDLocked=T|DesignItemId=MT41K256M16TW-107:P|AllPinCount=57
    block[2] differs in /MT41K256M16TW-107_P/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-25|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=16|NAME=Data Bus Width|UNIQUEID=CFSPQIBH
      file2 text: |RECORD=41|IndexInSheet=1|OwnerPartId=-1|Location.X=-5|Location.Y=-25|Color=8388608|FontID=1|IsHidden=T|Text=16|Name=Data Bus Width|UniqueID=CFSPQIBH
    block[3] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=2|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=450|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=Digi-Key|NAME=Supplier 1|READONLYSTATE=3|UNIQUEID=WGUGRNLO
      file2 text: |RECORD=41|IndexInSheet=2|OwnerPartId=-1|Location.X=-5|Location.Y=450|Color=8388608|FontID=1|IsHidden=T|Text=Digi-Key|Name=Supplier 1|ReadOnlyState=3|UniqueID=WGUGRNLO
    block[4] differs in /MT41K256M16TW-107_P/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=14|ISNOTACCESIBLE=T|INDEXINSHEET=3|OWNERPARTID=1|LOCATION.X=80|LOCATION.Y=30|CORNER.X=160|CORNER.Y=450|COLOR=128|AREACOLOR=11599871|ISSOLID=T
      file2 text: |RECORD=14|IsNotAccesible=T|IndexInSheet=3|OwnerPartId=1|Location.X=80|Location.Y=30|Corner.X=160|Corner.Y=450|Color=128|AreaColor=11599871|IsSolid=T
    block[9] differs in /MT41K256M16TW-107_P/Data: text(167 bytes) vs text(167 bytes)
      file1 text: |RECORD=41|OWNERINDEX=8|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=167|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=TYFXLWIQ
      file2 text: |RECORD=41|OwnerIndex=8|OwnerPartId=-1|Location.X=82|Location.Y=167|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=TYFXLWIQ
    block[13] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=12|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=337|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=CEXBFLDT
      file2 text: |RECORD=41|OwnerIndex=12|OwnerPartId=-1|Location.X=82|Location.Y=337|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=CEXBFLDT
    block[15] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=14|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=327|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=RMPUOJBI
      file2 text: |RECORD=41|OwnerIndex=14|OwnerPartId=-1|Location.X=82|Location.Y=327|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=RMPUOJBI
    block[17] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=16|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=317|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=RKLSPNFN
      file2 text: |RECORD=41|OwnerIndex=16|OwnerPartId=-1|Location.X=82|Location.Y=317|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=RKLSPNFN
    block[19] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=18|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=67|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DM_INPUT_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=WFMEXIPR
      file2 text: |RECORD=41|OwnerIndex=18|OwnerPartId=-1|Location.X=82|Location.Y=67|Color=8388608|FontID=1|IsHidden=T|Text=DM_INPUT_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=WFMEXIPR
    block[21] differs in /MT41K256M16TW-107_P/Data: text(170 bytes) vs text(170 bytes)
      file1 text: |RECORD=41|OWNERINDEX=20|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=127|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DM_INPUT_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=SVCESERC
      file2 text: |RECORD=41|OwnerIndex=20|OwnerPartId=-1|Location.X=82|Location.Y=127|Color=8388608|FontID=1|IsHidden=T|Text=DM_INPUT_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=SVCESERC
    block[23] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=22|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=267|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=SDJCBGLB
      file2 text: |RECORD=41|OwnerIndex=22|OwnerPartId=-1|Location.X=82|Location.Y=267|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=SDJCBGLB
    block[25] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=24|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=257|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=MCDAGYCS
      file2 text: |RECORD=41|OwnerIndex=24|OwnerPartId=-1|Location.X=82|Location.Y=257|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=MCDAGYCS
    block[27] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=26|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=247|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=JBCQIUGS
      file2 text: |RECORD=41|OwnerIndex=26|OwnerPartId=-1|Location.X=82|Location.Y=247|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=JBCQIUGS
    block[29] differs in /MT41K256M16TW-107_P/Data: text(162 bytes) vs text(162 bytes)
      file1 text: |RECORD=41|OWNERINDEX=28|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=227|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=RESET_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=HUUXAHJG
      file2 text: |RECORD=41|OwnerIndex=28|OwnerPartId=-1|Location.X=82|Location.Y=227|Color=8388608|FontID=1|IsHidden=T|Text=RESET_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=HUUXAHJG
    block[33] differs in /MT41K256M16TW-107_P/Data: text(167 bytes) vs text(167 bytes)
      file1 text: |RECORD=41|OWNERINDEX=32|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=197|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=CLKIN_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=VMUGLFYM
      file2 text: |RECORD=41|OwnerIndex=32|OwnerPartId=-1|Location.X=82|Location.Y=197|Color=8388608|FontID=1|IsHidden=T|Text=CLKIN_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=VMUGLFYM
    block[35] differs in /MT41K256M16TW-107_P/Data: text(167 bytes) vs text(167 bytes)
      file1 text: |RECORD=41|OWNERINDEX=34|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=187|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=CLKIN_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=GRXEQEFH
      file2 text: |RECORD=41|OwnerIndex=34|OwnerPartId=-1|Location.X=82|Location.Y=187|Color=8388608|FontID=1|IsHidden=T|Text=CLKIN_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=GRXEQEFH
    block[37] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=36|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=177|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=PHEQGSSI
      file2 text: |RECORD=41|OwnerIndex=36|OwnerPartId=-1|Location.X=82|Location.Y=177|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=PHEQGSSI
    block[39] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=38|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=437|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=EOWAYURF
      file2 text: |RECORD=41|OwnerIndex=38|OwnerPartId=-1|Location.X=192|Location.Y=437|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=EOWAYURF
    block[41] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=40|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=427|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=IVWSISDH
      file2 text: |RECORD=41|OwnerIndex=40|OwnerPartId=-1|Location.X=192|Location.Y=427|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=IVWSISDH
    block[43] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=42|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=417|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=CCKOCPCJ
      file2 text: |RECORD=41|OwnerIndex=42|OwnerPartId=-1|Location.X=192|Location.Y=417|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=CCKOCPCJ
    block[44] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=42|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=417|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=OVJIFIAE
      file2 text: |RECORD=41|OwnerIndex=42|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=417|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=OVJIFIAE
    block[46] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=45|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=407|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=VCYFJREJ
      file2 text: |RECORD=41|OwnerIndex=45|OwnerPartId=-1|Location.X=192|Location.Y=407|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=VCYFJREJ
    block[47] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=45|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=407|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=WRSVCCEA
      file2 text: |RECORD=41|OwnerIndex=45|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=407|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=WRSVCCEA
    block[49] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=48|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=397|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=CHKDEDET
      file2 text: |RECORD=41|OwnerIndex=48|OwnerPartId=-1|Location.X=192|Location.Y=397|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=CHKDEDET
    block[51] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=50|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=387|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=DXTEQSVW
      file2 text: |RECORD=41|OwnerIndex=50|OwnerPartId=-1|Location.X=192|Location.Y=387|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=DXTEQSVW
    block[53] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=52|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=377|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=TGDSHPLS
      file2 text: |RECORD=41|OwnerIndex=52|OwnerPartId=-1|Location.X=192|Location.Y=377|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=TGDSHPLS
    block[55] differs in /MT41K256M16TW-107_P/Data: text(170 bytes) vs text(170 bytes)
      file1 text: |RECORD=41|OWNERINDEX=54|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=367|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=NRPOKEDQ
      file2 text: |RECORD=41|OwnerIndex=54|OwnerPartId=-1|Location.X=192|Location.Y=367|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=NRPOKEDQ
    block[56] differs in /MT41K256M16TW-107_P/Data: text(184 bytes) vs text(184 bytes)
      file1 text: |RECORD=41|OWNERINDEX=54|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=367|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=CYUJUHED
      file2 text: |RECORD=41|OwnerIndex=54|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=367|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=CYUJUHED
    block[58] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=57|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=357|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=XMDBFERR
      file2 text: |RECORD=41|OwnerIndex=57|OwnerPartId=-1|Location.X=192|Location.Y=357|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=XMDBFERR
    block[60] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=59|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=347|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=HRSQWGEF
      file2 text: |RECORD=41|OwnerIndex=59|OwnerPartId=-1|Location.X=192|Location.Y=347|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=HRSQWGEF
    block[62] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=61|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=337|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=XSIYGVLM
      file2 text: |RECORD=41|OwnerIndex=61|OwnerPartId=-1|Location.X=192|Location.Y=337|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=XSIYGVLM
    block[64] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=63|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=327|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=KKGDSITU
      file2 text: |RECORD=41|OwnerIndex=63|OwnerPartId=-1|Location.X=192|Location.Y=327|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=KKGDSITU
    block[66] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=65|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=317|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=UTEIQTCN
      file2 text: |RECORD=41|OwnerIndex=65|OwnerPartId=-1|Location.X=192|Location.Y=317|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=UTEIQTCN
    block[68] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=67|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=307|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=YBAAKCUB
      file2 text: |RECORD=41|OwnerIndex=67|OwnerPartId=-1|Location.X=192|Location.Y=307|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=YBAAKCUB
    block[70] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=69|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=297|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT1_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=HWVBFGKC
      file2 text: |RECORD=41|OwnerIndex=69|OwnerPartId=-1|Location.X=192|Location.Y=297|Color=8388608|FontID=1|IsHidden=T|Text=INPUT1_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=HWVBFGKC
    block[72] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=71|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=257|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=UICMBNAC
      file2 text: |RECORD=41|OwnerIndex=71|OwnerPartId=-1|Location.X=192|Location.Y=257|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=UICMBNAC
    block[73] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=71|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=257|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=ATTNOHAH
      file2 text: |RECORD=41|OwnerIndex=71|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=257|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=ATTNOHAH
    block[75] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=74|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=247|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=DFIYKYDW
      file2 text: |RECORD=41|OwnerIndex=74|OwnerPartId=-1|Location.X=192|Location.Y=247|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=DFIYKYDW
    block[76] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=74|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=247|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=GXTTJWKC
      file2 text: |RECORD=41|OwnerIndex=74|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=247|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=GXTTJWKC
    block[78] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=77|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=237|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=SQOFETPK
      file2 text: |RECORD=41|OwnerIndex=77|OwnerPartId=-1|Location.X=192|Location.Y=237|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=SQOFETPK
    block[79] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=77|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=237|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=IQBNBCTW
      file2 text: |RECORD=41|OwnerIndex=77|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=237|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=IQBNBCTW
    block[81] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=80|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=227|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=LGUBEHEF
      file2 text: |RECORD=41|OwnerIndex=80|OwnerPartId=-1|Location.X=192|Location.Y=227|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=LGUBEHEF
    block[82] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=80|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=227|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=ANBQYPNQ
      file2 text: |RECORD=41|OwnerIndex=80|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=227|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=ANBQYPNQ
    block[84] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=83|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=217|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=AJLBADIS
      file2 text: |RECORD=41|OwnerIndex=83|OwnerPartId=-1|Location.X=192|Location.Y=217|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=AJLBADIS
    block[85] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=83|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=217|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=BGRYPEFW
      file2 text: |RECORD=41|OwnerIndex=83|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=217|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=BGRYPEFW
    block[87] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=86|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=207|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=VBDHSWMX
      file2 text: |RECORD=41|OwnerIndex=86|OwnerPartId=-1|Location.X=192|Location.Y=207|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=VBDHSWMX
    block[88] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=86|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=207|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=EWMTAXCL
      file2 text: |RECORD=41|OwnerIndex=86|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=207|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=EWMTAXCL
    block[90] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=89|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=197|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=VOFYTQRS
      file2 text: |RECORD=41|OwnerIndex=89|OwnerPartId=-1|Location.X=192|Location.Y=197|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=VOFYTQRS
    block[91] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=89|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=197|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=CNXULKXJ
      file2 text: |RECORD=41|OwnerIndex=89|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=197|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=CNXULKXJ
    block[93] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=92|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=187|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=NTGHNUWW
      file2 text: |RECORD=41|OwnerIndex=92|OwnerPartId=-1|Location.X=192|Location.Y=187|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=NTGHNUWW
    block[94] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=92|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=187|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=TSDUHIJS
      file2 text: |RECORD=41|OwnerIndex=92|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=187|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=TSDUHIJS
    block[96] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=95|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=157|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=LPQKSENP
      file2 text: |RECORD=41|OwnerIndex=95|OwnerPartId=-1|Location.X=192|Location.Y=157|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=LPQKSENP
    block[97] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=95|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=157|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=YVTNLSDF
      file2 text: |RECORD=41|OwnerIndex=95|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=157|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=YVTNLSDF
    block[99] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=98|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=147|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=QHFGSXLB
      file2 text: |RECORD=41|OwnerIndex=98|OwnerPartId=-1|Location.X=192|Location.Y=147|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=QHFGSXLB
    block[100] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=98|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=147|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=PDCIPGSW
      file2 text: |RECORD=41|OwnerIndex=98|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=147|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=PDCIPGSW
    block[102] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=101|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=137|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=BQFFAWWE
      file2 text: |RECORD=41|OwnerIndex=101|OwnerPartId=-1|Location.X=192|Location.Y=137|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=BQFFAWWE
    block[103] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=101|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=137|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=FWTUXHLY
      file2 text: |RECORD=41|OwnerIndex=101|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=137|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=FWTUXHLY
    block[105] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=104|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=127|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=XBMURVGH
      file2 text: |RECORD=41|OwnerIndex=104|OwnerPartId=-1|Location.X=192|Location.Y=127|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=XBMURVGH
    block[106] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=104|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=127|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=LSVLGGKK
      file2 text: |RECORD=41|OwnerIndex=104|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=127|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=LSVLGGKK
    block[108] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=107|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=117|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=HWCGTPGV
      file2 text: |RECORD=41|OwnerIndex=107|OwnerPartId=-1|Location.X=192|Location.Y=117|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=HWCGTPGV
    block[109] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=107|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=117|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=ISRLTOJM
      file2 text: |RECORD=41|OwnerIndex=107|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=117|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=ISRLTOJM
    block[111] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=110|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=107|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=LLLJWDSB
      file2 text: |RECORD=41|OwnerIndex=110|OwnerPartId=-1|Location.X=192|Location.Y=107|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=LLLJWDSB
    block[112] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=110|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=107|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=XPAJGDTT
      file2 text: |RECORD=41|OwnerIndex=110|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=107|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=XPAJGDTT
    block[114] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=113|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=97|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=HKFBJVQP
      file2 text: |RECORD=41|OwnerIndex=113|OwnerPartId=-1|Location.X=192|Location.Y=97|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=HKFBJVQP
    block[115] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=113|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=97|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=PXQSOFNN
      file2 text: |RECORD=41|OwnerIndex=113|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=97|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=PXQSOFNN
    block[117] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=116|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=87|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=TUEEYUXJ
      file2 text: |RECORD=41|OwnerIndex=116|OwnerPartId=-1|Location.X=192|Location.Y=87|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=TUEEYUXJ
    block[118] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=116|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=87|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQ_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=PGPUEWEQ
      file2 text: |RECORD=41|OwnerIndex=116|IndexInSheet=1|OwnerPartId=-1|Location.X=192|Location.Y=87|Color=8388608|FontID=1|IsHidden=T|Text=DQ_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=PGPUEWEQ
    block[120] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=119|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=107|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=KIHCFVPB
      file2 text: |RECORD=41|OwnerIndex=119|OwnerPartId=-1|Location.X=82|Location.Y=107|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=KIHCFVPB
    block[121] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=119|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=107|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=ILYGLBNB
      file2 text: |RECORD=41|OwnerIndex=119|IndexInSheet=1|OwnerPartId=-1|Location.X=82|Location.Y=107|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=ILYGLBNB
    block[123] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=122|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=47|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=RLMJTTLV
      file2 text: |RECORD=41|OwnerIndex=122|OwnerPartId=-1|Location.X=82|Location.Y=47|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=RLMJTTLV
    block[124] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=122|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=47|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=WRUSHRXR
      file2 text: |RECORD=41|OwnerIndex=122|IndexInSheet=1|OwnerPartId=-1|Location.X=82|Location.Y=47|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=WRUSHRXR
    block[126] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=125|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=117|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=NQTNYOFH
      file2 text: |RECORD=41|OwnerIndex=125|OwnerPartId=-1|Location.X=82|Location.Y=117|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=NQTNYOFH
    block[127] differs in /MT41K256M16TW-107_P/Data: text(186 bytes) vs text(186 bytes)
      file1 text: |RECORD=41|OWNERINDEX=125|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=117|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=NTFUICYY
      file2 text: |RECORD=41|OwnerIndex=125|IndexInSheet=1|OwnerPartId=-1|Location.X=82|Location.Y=117|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=NTFUICYY
    block[129] differs in /MT41K256M16TW-107_P/Data: text(168 bytes) vs text(168 bytes)
      file1 text: |RECORD=41|OWNERINDEX=128|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=57|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=OPATENHR
      file2 text: |RECORD=41|OwnerIndex=128|OwnerPartId=-1|Location.X=82|Location.Y=57|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=OPATENHR
    block[130] differs in /MT41K256M16TW-107_P/Data: text(185 bytes) vs text(185 bytes)
      file1 text: |RECORD=41|OWNERINDEX=128|INDEXINSHEET=1|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=57|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=DQS_34_1066_tp_out.mac|NAME=SI_OUTPUT_MODEL|UNIQUEID=BGDLNYMQ
      file2 text: |RECORD=41|OwnerIndex=128|IndexInSheet=1|OwnerPartId=-1|Location.X=82|Location.Y=57|Color=8388608|FontID=1|IsHidden=T|Text=DQS_34_1066_tp_out.mac|Name=SI_OUTPUT_MODEL|UniqueID=BGDLNYMQ
    block[131] differs in /MT41K256M16TW-107_P/Data: text(175 bytes) vs text(175 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=59|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=450|COLOR=8388608|FONTID=1|TEXT=557-1741-1-ND|NAME=Supplier Part Number 1|READONLYSTATE=3|UNIQUEID=QJWHEDIY
      file2 text: |RECORD=41|IndexInSheet=59|OwnerPartId=-1|Location.X=-5|Location.Y=450|Color=8388608|FontID=1|Text=557-1741-1-ND|Name=Supplier Part Number 1|ReadOnlyState=3|UniqueID=QJWHEDIY
    block[133] differs in /MT41K256M16TW-107_P/Data: text(169 bytes) vs text(169 bytes)
      file1 text: |RECORD=41|OWNERINDEX=132|OWNERPARTID=-1|LOCATION.X=82|LOCATION.Y=307|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=INPUT2_1066_tp_in.mac|NAME=SI_INPUT_MODEL|UNIQUEID=EHHPETCU
      file2 text: |RECORD=41|OwnerIndex=132|OwnerPartId=-1|Location.X=82|Location.Y=307|Color=8388608|FontID=1|IsHidden=T|Text=INPUT2_1066_tp_in.mac|Name=SI_INPUT_MODEL|UniqueID=EHHPETCU
    block[134] differs in /MT41K256M16TW-107_P/Data: text(150 bytes) vs text(150 bytes)
      file1 text: |RECORD=34|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=5|COLOR=8388608|FONTID=1|TEXT=U?|NAME=Designator|READONLYSTATE=1|UNIQUEID=CENQDSMJ
      file2 text: |RECORD=34|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=5|Color=8388608|FontID=1|Text=U?|Name=Designator|ReadOnlyState=1|UniqueID=CENQDSMJ
    block[135] differs in /MT41K256M16TW-107_P/Data: text(132 bytes) vs text(132 bytes)
      file1 text: |RECORD=41|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=-5|LOCATION.Y=-15|COLOR=8388608|FONTID=1|TEXT=*|NAME=Comment|UNIQUEID=UQIMNBQJ
      file2 text: |RECORD=41|IndexInSheet=-1|OwnerPartId=-1|Location.X=-5|Location.Y=-15|Color=8388608|FontID=1|Text=*|Name=Comment|UniqueID=UQIMNBQJ
    block[137] differs in /MT41K256M16TW-107_P/Data: text(282 bytes) vs text(282 bytes)
      file1 text: |RECORD=45|OWNERINDEX=136|INDEXINSHEET=-1|DESCRIPTION=BGA, 96-Leads, Body 8.00x14.00mmx1.20mm, Pitch 0.80mm|MODELNAME=BGA96C80P9X16_800X1400X120|MODELTYPE=PCBLIB|DATAFILECOUNT=1|MODELDATAFILEENTITY0=BGA96C80P9X16_800X1400X120|MODELDATAFILEKIND0=PCBLib|ISCURRENT=T|UNIQUEID=VENIRCTX
      file2 text: |RECORD=45|OwnerIndex=136|IndexInSheet=-1|Description=BGA, 96-Leads, Body 8.00x14.00mmx1.20mm, Pitch 0.80mm|ModelName=BGA96C80P9X16_800X1400X120|ModelType=PCBLIB|DatafileCount=1|ModelDatafileEntity0=BGA96C80P9X16_800X1400X120|ModelDatafileKind0=PCBLib|IsCurrent=T|UniqueID=VENIRCTX
    block[138] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=46|OWNERINDEX=137
      file2 text: |RECORD=46|OwnerIndex=137
    block[139] differs in /MT41K256M16TW-107_P/Data: text(141 bytes) vs text(141 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|DESINTF=VDDQ|DESIMPCOUNT=8|DESIMP0=A1|DESIMP1=A8|DESIMP2=C1|DESIMP3=C9|DESIMP4=D2|DESIMP5=E9|DESIMP6=F1|DESIMP7=H9
      file2 text: |RECORD=47|OwnerIndex=138|DesIntf=VDDQ|DesImpCount=8|DesImp0=A1|DesImp1=A8|DesImp2=C1|DesImp3=C9|DesImp4=D2|DesImp5=E9|DesImp6=F1|DesImp7=H9
    block[140] differs in /MT41K256M16TW-107_P/Data: text(121 bytes) vs text(121 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|INDEXINSHEET=1|DESINTF=NC|DESIMPCOUNT=5|DESIMP0=J1|DESIMP1=J9|DESIMP2=L1|DESIMP3=L9|DESIMP4=M7
      file2 text: |RECORD=47|OwnerIndex=138|IndexInSheet=1|DesIntf=NC|DesImpCount=5|DesImp0=J1|DesImp1=J9|DesImp2=L1|DesImp3=L9|DesImp4=M7
    block[141] differs in /MT41K256M16TW-107_P/Data: text(167 bytes) vs text(167 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|INDEXINSHEET=2|DESINTF=VSSQ|DESIMPCOUNT=9|DESIMP0=B1|DESIMP1=B9|DESIMP2=D1|DESIMP3=D8|DESIMP4=E2|DESIMP5=E8|DESIMP6=F9|DESIMP7=G1|DESIMP8=G9
      file2 text: |RECORD=47|OwnerIndex=138|IndexInSheet=2|DesIntf=VSSQ|DesImpCount=9|DesImp0=B1|DesImp1=B9|DesImp2=D1|DesImp3=D8|DesImp4=E2|DesImp5=E8|DesImp6=F9|DesImp7=G1|DesImp8=G9
    block[142] differs in /MT41K256M16TW-107_P/Data: text(202 bytes) vs text(202 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|INDEXINSHEET=3|DESINTF=VSS|DESIMPCOUNT=12|DESIMP0=A9|DESIMP1=B3|DESIMP2=E1|DESIMP3=G8|DESIMP4=J2|DESIMP5=J8|DESIMP6=M1|DESIMP7=M9|DESIMP8=P1|DESIMP9=P9|DESIMP10=T1|DESIMP11=T9
      file2 text: |RECORD=47|OwnerIndex=138|IndexInSheet=3|DesIntf=VSS|DesImpCount=12|DesImp0=A9|DesImp1=B3|DesImp2=E1|DesImp3=G8|DesImp4=J2|DesImp5=J8|DesImp6=M1|DesImp7=M9|DesImp8=P1|DesImp9=P9|DesImp10=T1|DesImp11=T9
    block[143] differs in /MT41K256M16TW-107_P/Data: text(166 bytes) vs text(166 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|INDEXINSHEET=4|DESINTF=VDD|DESIMPCOUNT=9|DESIMP0=B2|DESIMP1=D9|DESIMP2=G7|DESIMP3=K2|DESIMP4=K8|DESIMP5=N1|DESIMP6=N9|DESIMP7=R1|DESIMP8=R9
      file2 text: |RECORD=47|OwnerIndex=138|IndexInSheet=4|DesIntf=VDD|DesImpCount=9|DesImp0=B2|DesImp1=D9|DesImp2=G7|DesImp3=K2|DesImp4=K8|DesImp5=N1|DesImp6=N9|DesImp7=R1|DesImp8=R9
    block[144] differs in /MT41K256M16TW-107_P/Data: text(77 bytes) vs text(77 bytes)
      file1 text: |RECORD=47|OWNERINDEX=138|INDEXINSHEET=5|DESINTF=p2|DESIMPCOUNT=1|DESIMP0=P2
      file2 text: |RECORD=47|OwnerIndex=138|IndexInSheet=5|DesIntf=p2|DesImpCount=1|DesImp0=P2
    block[145] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=48|OWNERINDEX=137
      file2 text: |RECORD=48|OwnerIndex=137
    block[146] differs in /MT41K256M16TW-107_P/Data: text(140 bytes) vs text(140 bytes)
      file1 text: |RECORD=45|OWNERINDEX=136|INDEXINSHEET=-1|DESCRIPTION=Model Description|MODELNAME=1066_34_no_odt|MODELTYPE=SI|ISCURRENT=T|UNIQUEID=JCHESYBX
      file2 text: |RECORD=45|OwnerIndex=136|IndexInSheet=-1|Description=Model Description|ModelName=1066_34_no_odt|ModelType=SI|IsCurrent=T|UniqueID=JCHESYBX
    block[147] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=46|OWNERINDEX=146
      file2 text: |RECORD=46|OwnerIndex=146
    block[148] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=48|OWNERINDEX=146
      file2 text: |RECORD=48|OwnerIndex=146
    block[149] differs in /MT41K256M16TW-107_P/Data: text(156 bytes) vs text(156 bytes)
      file1 text: |RECORD=41|OWNERINDEX=148|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=30|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=IC|NAME=TYPE|UNIQUEID=BJGMUGHV
      file2 text: |RECORD=41|OwnerIndex=148|IndexInSheet=-1|OwnerPartId=-1|Location.X=192|Location.Y=30|Color=8388608|FontID=1|IsHidden=T|Text=IC|Name=TYPE|UniqueID=BJGMUGHV
    block[150] differs in /MT41K256M16TW-107_P/Data: text(161 bytes) vs text(161 bytes)
      file1 text: |RECORD=41|OWNERINDEX=148|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=30|COLOR=8388608|FONTID=1|ISHIDDEN=T|TEXT=Unknown|NAME=TECH|UNIQUEID=VSQGMFPF
      file2 text: |RECORD=41|OwnerIndex=148|IndexInSheet=-1|OwnerPartId=-1|Location.X=192|Location.Y=30|Color=8388608|FontID=1|IsHidden=T|Text=Unknown|Name=TECH|UniqueID=VSQGMFPF
    block[151] differs in /MT41K256M16TW-107_P/Data: text(324 bytes) vs text(324 bytes)
      file1 text: |RECORD=45|OWNERINDEX=136|INDEXINSHEET=-1|DESCRIPTION=1066Mbps_32ohm_no_odt|MODELNAME=MT41K256M16TW|MODELTYPE=IBIS|DATAFILECOUNT=1|MODELDATAFILE0=C:\Users\Zero.000\Documents\Altium Designer\IBIS Models\MT41K256M16TW-107 P\v00h_1p35.ibs|MODELDATAFILEENTITY0=MT41K256M16TW|MODELDATAFILEKIND0=ibs|ISCURRENT=T|UNIQUEID=LBPHQSTD
      file2 text: |RECORD=45|OwnerIndex=136|IndexInSheet=-1|Description=1066Mbps_32ohm_no_odt|ModelName=MT41K256M16TW|ModelType=IBIS|DatafileCount=1|ModelDatafile0=C:\Users\Zero.000\Documents\Altium Designer\IBIS Models\MT41K256M16TW-107 P\v00h_1p35.ibs|ModelDatafileEntity0=MT41K256M16TW|ModelDatafileKind0=ibs|IsCurrent=T|UniqueID=LBPHQSTD
    block[152] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=46|OWNERINDEX=151
      file2 text: |RECORD=46|OwnerIndex=151
    block[153] differs in /MT41K256M16TW-107_P/Data: text(26 bytes) vs text(26 bytes)
      file1 text: |RECORD=48|OWNERINDEX=151
      file2 text: |RECORD=48|OwnerIndex=151
    block[154] differs in /MT41K256M16TW-107_P/Data: text(159 bytes) vs text(159 bytes)
      file1 text: |RECORD=41|OWNERINDEX=153|INDEXINSHEET=-1|OWNERPARTID=-1|LOCATION.X=192|LOCATION.Y=30|COLOR=8388608|FONTID=1|TEXT=Typical|NAME=DriveStrength|UNIQUEID=YOGUBRXP
      file2 text: |RECORD=41|OwnerIndex=153|IndexInSheet=-1|OwnerPartId=-1|Location.X=192|Location.Y=30|Color=8388608|FontID=1|Text=Typical|Name=DriveStrength|UniqueID=YOGUBRXP
  stream DIFFERS: /MT41K256M16TW-107_P/PinPackageLength
    first diff at stream offset 0x0000001e (30)
    file1 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 45 49 47 48 54 3d 35 37 
    file2 hex [0x00000016+]: 4c 65 6e 67 74 68 7c 57 65 69 67 68 74 3d 35 37 
    block[0] differs in /MT41K256M16TW-107_P/PinPackageLength: text(35 bytes) vs text(35 bytes)
      file1 text: |HEADER=PinPackageLength|WEIGHT=57
      file2 text: |HEADER=PinPackageLength|Weight=57
  stream DIFFERS: /MT41K256M16TW-107_P/PinSymbolLineWidth
    first diff at stream offset 0x00000020 (32)
    file1 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 45 49 47 48 54 3d 35 37 
    file2 hex [0x00000018+]: 65 57 69 64 74 68 7c 57 65 69 67 68 74 3d 35 37 
    block[0] differs in /MT41K256M16TW-107_P/PinSymbolLineWidth: text(37 bytes) vs text(37 bytes)
      file1 text: |HEADER=PinSymbolLineWidth|WEIGHT=57
      file2 text: |HEADER=PinSymbolLineWidth|Weight=57
  MISSING in file1: /SectionKeys
  stream OK: /Storage (25 bytes)
```

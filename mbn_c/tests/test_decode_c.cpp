#include <gtest/gtest.h>

#include <cstdint>
#include <cstdio>
#include <cstring>  // For memcpy
#include <iostream>
#include <vector>

#include "mbn_c.h"

TEST(DecoderTests, test_decode_buffer) {
  // Prepare test records
  Mbp1Msg record1 =
      create_mbp1(1, 1622471124, 1000, 10, 84, 65, 0, 130, 123456789098765,
                  12345, 123456, 0, 1, 2, 2, 2, 1, 3);

  std::vector<RecordData> records;
  records.push_back(RecordData{.mbp1 = record1});

  // Create the CRecordEncoder
  CRecordEncoder* encoder = create_record_encoder();
  ASSERT_NE(encoder, nullptr) << "Failed to create CRecordEncoder";

  // Encode the records
  int result = encode_records(encoder, records.data(), records.size());
  EXPECT_EQ(result, 0) << "Failed to encode records";

  // Retrieve encoded data
  size_t encoded_size = 0;
  get_encoded_data(encoder, nullptr, &encoded_size);  // Query size first
  ASSERT_GT(encoded_size, 0) << "Encoded data size is zero";

  std::vector<uint8_t> encoded_data(encoded_size);
  get_encoded_data(encoder, encoded_data.data(), &encoded_size);

  // Destroy the encoder
  destroy_record_encoder(encoder);

  // Test
  CRecordDecoder* decoder =
      create_buffer_decoder(encoded_data.data(), encoded_size);

  size_t decoded_size = 0;
  RecordData* decoded_records;
  decoded_records = decode_records(decoder, &decoded_size);

  if (!decoded_records) {
    std::cerr << "Failed to decode records!" << std::endl;
    return;
  }

  // Iterate over the array of decoded records
  // ASSERT_EQ(decoded_records[0].field1, record1.field1);
  // ASSERT_EQ(decoded_records[0].field2, record1.field2);
  // ASSERT_EQ(decoded_records[0].field3, record1.field3);
  // ASSERT_EQ(decoded_records[1].field1, record2.field1);
  // ASSERT_EQ(decoded_records[1].field2, record2.field2);
  // ASSERT_EQ(decoded_records[1].field3, record2.field3);

  for (size_t i = 0; i < decoded_size; ++i) {
    RecordData record = decoded_records[i];
    const Mbp1Msg* msg = get_mbp1(&record);

    if (msg == nullptr) {  // Check for null pointer
      std::cerr << "Error: Failed to get Mbp1Msg from record " << i
                << std::endl;
      continue;  // Skip this record and move to the next
    }

    // Safely output the data
    std::cout << "ts_event " << msg->hd.ts_event << "\n"
              << "instrument_id " << msg->hd.instrument_id << "\n"
              << "price " << msg->price << "\n"
              << "size " << msg->size << "\n"
              << "action " << msg->action << "\n"
              << "side " << msg->side << "\n"
              << "depth " << msg->depth << "\n"
              << "flag " << msg->flags << "\n"
              << "ts_recv" << msg->ts_recv << "\n"
              << "ts_in_delta " << msg->ts_in_delta << "\n"
              << "sequence " << msg->sequence << "\n"
              << "discriminator " << msg->discriminator << std::endl;
  }

  // Free the memory allocated by Rust
  destroy_record_decoder(decoder);
}

TEST(DecoderTests, test_decode_file) {
  // Prepare test records
  Mbp1Msg record1 =
      create_mbp1(1, 1622471124, 1000, 10, 84, 65, 0, 130, 123456789098765,
                  12345, 123456, 0, 1, 2, 2, 2, 1, 3);

  std::vector<RecordData> records;
  records.push_back(RecordData{.mbp1 = record1});

  // Create the CRecordEncoder
  CRecordEncoder* encoder = create_record_encoder();
  ASSERT_NE(encoder, nullptr) << "Failed to create CRecordEncoder";

  // Encode the records
  int result = encode_records(encoder, records.data(), records.size());
  EXPECT_EQ(result, 0) << "Failed to encode records";

  // Retrieve encoded data
  size_t encoded_size = 0;
  get_encoded_data(encoder, nullptr, &encoded_size);  // Query size first
  ASSERT_GT(encoded_size, 0) << "Encoded data size is zero";

  std::vector<uint8_t> encoded_data(encoded_size);
  get_encoded_data(encoder, encoded_data.data(), &encoded_size);

  // Write to file
  const char* file = "../tests/test_decode.bin";
  write_buffer_to_file(encoder, file, false);

  // Destroy the encoder
  destroy_record_encoder(encoder);

  // Test
  CRecordDecoder* decoder = create_file_decoder(file);

  size_t decoded_size = 0;
  RecordData* decoded_records;
  decoded_records = decode_records(decoder, &decoded_size);

  if (!decoded_records) {
    std::cerr << "Failed to decode records!" << std::endl;
    return;
  }

  // Iterate over the array of decoded records
  // ASSERT_EQ(decoded_records[0].field1, record1.field1);
  // ASSERT_EQ(decoded_records[0].field2, record1.field2);
  // ASSERT_EQ(decoded_records[0].field3, record1.field3);
  // ASSERT_EQ(decoded_records[1].field1, record2.field1);
  // ASSERT_EQ(decoded_records[1].field2, record2.field2);
  // ASSERT_EQ(decoded_records[1].field3, record2.field3);

  for (size_t i = 0; i < decoded_size; ++i) {
    RecordData record = decoded_records[i];
    const Mbp1Msg* msg = get_mbp1(&record);

    if (msg == nullptr) {  // Check for null pointer
      std::cerr << "Error: Failed to get Mbp1Msg from record " << i
                << std::endl;
      continue;  // Skip this record and move to the next
    }

    // Safely output the data
    std::cout << "ts_event " << msg->hd.ts_event << "\n"
              << "instrument_id " << msg->hd.instrument_id << "\n"
              << "price " << msg->price << "\n"
              << "size " << msg->size << "\n"
              << "action " << msg->action << "\n"
              << "side " << msg->side << "\n"
              << "depth " << msg->depth << "\n"
              << "flag " << msg->flags << "\n"
              << "ts_recv" << msg->ts_recv << "\n"
              << "ts_in_delta " << msg->ts_in_delta << "\n"
              << "sequence " << msg->sequence << "\n"
              << "discriminator " << msg->discriminator << std::endl;
  }

  // Free the memory allocated by Rust
  destroy_record_decoder(decoder);
}

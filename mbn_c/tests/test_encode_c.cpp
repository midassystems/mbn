#include <gtest/gtest.h>

#include <cstdint>
#include <cstring>  // For memcpy
#include <iostream>
#include <vector>

#include "mbn_c.h"

TEST(EncoderTests, test_encoder_buffer) {
  // Prepare test records
  Mbp1Msg record1 =
      create_mbp1(1, 1622471124, 1000, 10, 1, 1, 0, 0, 123456789098765, 12345,
                  123456, 0, 1, 2, 2, 2, 1, 3);

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

  // Validate the encoded data (optional, depends on the implementation)
  std::cout << "Encoded data: ";
  for (size_t i = 0; i < encoded_size; ++i) {
    std::cout << std::hex << static_cast<int>(encoded_data[i]) << " ";
  }
  std::cout << std::dec << std::endl;

  // Destroy the encoder
  destroy_record_encoder(encoder);
}

TEST(EncoderTests, test_encode_to_file) {
  // Prepare test records
  Mbp1Msg record1 =
      create_mbp1(1, 1622471124, 1000, 10, 1, 1, 0, 0, 123456789098765, 12345,
                  123456, 0, 1, 2, 2, 2, 1, 3);

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
  const char* file = "../tests/test_encode_records.bin";
  write_buffer_to_file(encoder, file, false);

  // Destroy the encoder
  destroy_record_encoder(encoder);
}
